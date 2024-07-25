use std::{
    net::ToSocketAddrs,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use async_std::{io, net::UdpSocket};
use futures::join;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    util::{TargetAddr, ToTargetAddr},
    RemoteMsg, RemoteToServer, ServerMsg, TunnelReader, TunnelWriter,
};

// 5 minutes
const SERVER_UDP_READ_TIMEOUT: Duration = Duration::from_secs(60 * 5);

pub async fn tcp_tunnel_port_task(
    target_addr: TargetAddr,
    reader_tunnel: TunnelReader<ServerMsg, RemoteMsg>,
    mut writer_tunnel: TunnelWriter<ServerMsg>,
) {
    trace!("tcp_runnel_port_task: start");
    let id = reader_tunnel.get_id();
    let target_addr_clone = target_addr.clone();
    let stream = match target_addr {
        TargetAddr::Domain(domain_addr, port) => TcpStream::connect((domain_addr, port)).await.ok(),
        TargetAddr::Ip(ip) => TcpStream::connect(ip).await.ok(),
    };

    let Some(stream) = stream else {
        error!("Connect {:?} failed", target_addr_clone);
        let _ = writer_tunnel
            .send(RemoteToServer::TcpConnectFailed(id).into())
            .await;
        let _ = writer_tunnel
            .send(RemoteToServer::ClosePort(id).into())
            .await;
        return;
    };

    match stream.local_addr() {
        Ok(bind_addr) => {
            debug!("Tcp connect local bind addr: {:?}", bind_addr);
            match bind_addr.to_target_addr() {
                Ok(bind_addr) => {
                    debug!(
                        "Tcp connect local bind addr to target addr: {:?}",
                        bind_addr
                    );
                    let _ = writer_tunnel
                        .send(RemoteToServer::TcpConnectSuccess(id, bind_addr).into())
                        .await;
                }
                Err(_) => {
                    error!("Bind addr to target addr failed");
                    let _ = writer_tunnel
                        .send(RemoteToServer::TcpConnectFailed(id).into())
                        .await;
                    let _ = writer_tunnel
                        .send(RemoteToServer::ClosePort(id).into())
                        .await;
                    return;
                }
            }
        }
        Err(_) => {
            error!("Get local addr failed");
            let _ = writer_tunnel
                .send(RemoteToServer::TcpConnectFailed(id).into())
                .await;
            let _ = writer_tunnel
                .send(RemoteToServer::ClosePort(id).into())
                .await;
            return;
        }
    }

    let (reader, writer) = stream.into_split();
    let r = read_remote_tcp(id, reader, writer_tunnel);
    let w = write_remote_tcp(writer, reader_tunnel);

    // tokio::select! {
    //     _ = r => {
    //         info!("Read remote tcp task end");
    //     }
    //     _ = w => {
    //         info!("Write remote tcp task end");
    //     }
    // };

    join!(w, r);
    info!("Server tunnel port tcp task end id: {}", id);
}

async fn read_remote_tcp(
    id: u32,
    mut stream: OwnedReadHalf,
    mut writer_tunnel: TunnelWriter<ServerMsg>,
) {
    loop {
        let mut buf = vec![0u8; 1024 * 40];
        match stream.read(&mut buf).await {
            Ok(0) => {
                info!("Tcp remote read 0");
                break;
            }

            Ok(n) => {
                buf.truncate(n);
                let msg = RemoteToServer::Data(id, Box::new(buf)).into();
                info!("Read remote tcp data len: {:?}", n);
                if writer_tunnel.send(msg).await.is_err() {
                    error!("Write tunnel error 0");
                    break;
                }
            }

            Err(_) => {
                warn!("Server Tcp Stream read error");
                break;
            }
        }
    }

    info!("Read remote tcp end");
    let msg = RemoteToServer::ClosePort(id).into();
    if writer_tunnel.send(msg).await.is_err() {
        warn!("Write tunnel error 1");
    }
}

async fn write_remote_tcp(
    mut stream: OwnedWriteHalf,
    mut reader_tunnel: TunnelReader<ServerMsg, RemoteMsg>,
) {
    loop {
        match reader_tunnel.read().await {
            Some(msg) => match msg {
                RemoteMsg::Data(data) => {
                    info!("Write remote tcp len: {:?}", data.len());
                    if stream.write_all(&data).await.is_err() {
                        warn!("Write remote tcp error");
                        break;
                    }
                }
                RemoteMsg::ClosePort(_) => {
                    warn!("Remote tcp close port");
                    break;
                }
                RemoteMsg::UdpData(_, _) => {
                    error!("Remote tcp get udp data");
                    break;
                }
            },
            None => {
                warn!("Read tunnel error");
                break;
            }
        }
    }

    reader_tunnel.rx = None;
    info!("Write remote tcp end");
    let id = reader_tunnel.get_id();
    let msg = RemoteToServer::ClosePort(id).into();
    if reader_tunnel.send(msg).await.is_err() {
        warn!("Write tunnel error 2");
    }
}

pub async fn udp_tunnel_port_task(
    reader_tunnel: TunnelReader<ServerMsg, RemoteMsg>,
    mut writer_tunnel: TunnelWriter<ServerMsg>,
) {
    trace!("udp_runnel_port_task: start");
    let id = reader_tunnel.get_id();

    // Only ipv4， 先留着，万一环境不支持 ipv6 就用这个
    // 只监听 ipv4 的话下面还有个 target_addr.set_ip 也要注释掉
    // let socket = match UdpSocket::bind("0.0.0.0:0").await {

    // Listen with UDP6 socket, so the client can connect to it with either
    // IPv4 or IPv6.
    let socket = match UdpSocket::bind("[::]:0").await {
        Ok(socket) => socket,
        Err(e) => {
            error!("Bind udp socket failed: {:?}", e);
            let _ = writer_tunnel
                .send(RemoteToServer::UdpAssociateFailed(id).into())
                .await;
            let _ = writer_tunnel
                .send(RemoteToServer::ClosePort(id).into())
                .await;
            return;
        }
    };
    info!(
        "server udp socket bind success: {}",
        socket.local_addr().unwrap()
    );

    let _ = writer_tunnel
        .send(RemoteToServer::UdpAssociateSuccess(id).into())
        .await;

    // 因为 read_remote_udp 里面用的 async_std 的 timeout，没有阻塞到 future 里，所以无法通过 tokio::select! 退出
    // 我们在这里用一个 atomic bool 来控制退出
    // 由于 udp 是无连接的，如果端口一直没有关闭，我们不知道什么时候关闭，所以特别依赖 close port 来关闭
    // TODO: 是否需要设置一个超时时间，如果超时时间到了，就关闭端口
    let running = AtomicBool::new(true);
    let r = read_remote_udp(&running, id, &socket, writer_tunnel);
    let w = write_remote_udp(&running, id, &socket, reader_tunnel);

    // tokio::select! {
    //     _ = r => {
    //         info!("Read remote udp task end");
    //     }
    //     _ = w => {
    //         info!("Write remote udp task end");
    //     }
    // };
    // 因为通过 running 来控制了退出，所以这里可以直接 join 等待两个任务结束
    join!(w, r);
    info!("Server tunnel port task udp end id: {}", id);
}

async fn read_remote_udp(
    running: &AtomicBool,
    id: u32,
    socket: &UdpSocket,
    mut writer_tunnel: TunnelWriter<ServerMsg>,
) {
    let mut now = Instant::now();
    loop {
        let mut buf = vec![0u8; 1024 * 40];
        if !running.load(Ordering::Relaxed) {
            break;
        }
        // 注意这里设置了超时，如果没有数据，就会返回超时错误，不会一直阻塞，因为测试时候发现一直阻塞，反而收不到数据
        // 有超时时间会收到数据，还不清楚原因
        // 由于这里 io 是 async_std 的，为了兼容好一点，socket 也用了 async_std 的 UdpSocket
        let res = io::timeout(Duration::from_millis(100), socket.recv_from(&mut buf)).await;
        match res {
            Ok((n, source)) => {
                now = Instant::now();
                let Ok(target_addr) = source.to_target_addr() else {
                    error!("Read remote udp target addr failed");
                    continue;
                };

                buf.truncate(n);
                let msg = RemoteToServer::UdpData(id, target_addr, Box::new(buf)).into();
                info!("Read remote udp data len: {:?}", n);
                if writer_tunnel.send(msg).await.is_err() {
                    error!("Write tunnel error 3");
                    break;
                }
            }
            Err(e) => {
                let duration = Instant::now() - now;
                if duration > SERVER_UDP_READ_TIMEOUT {
                    error!("Server Udp Socket id: {} read timeout: {:?}", id, duration);
                    break;
                }

                debug!(
                    "Server Udp Socket id: {} read error(maybe timeout): {:?}",
                    id, e
                );
                // break;
            }
        }
    }

    info!("Read remote udp end");
    running.store(false, Ordering::Relaxed);
    let msg = RemoteToServer::ClosePort(id).into();
    if writer_tunnel.send(msg).await.is_err() {
        error!("Write tunnel error 4");
    }
}

async fn write_remote_udp(
    running: &AtomicBool,
    id: u32,
    socket: &UdpSocket,
    mut reader_tunnel: TunnelReader<ServerMsg, RemoteMsg>,
) {
    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        match reader_tunnel.read().await {
            Some(msg) => match msg {
                RemoteMsg::UdpData(target_addr, data) => {
                    info!(
                        "Write remote udp data: {}, data len = {}",
                        target_addr,
                        data.len()
                    );
                    // 因为国内的 DNS 服务器有 DNS 污染，会返回错误的 IP 地址，所以不在客户端解析
                    let Ok(target_addr) = target_addr.resolve_dns().await else {
                        error!("Write remote udp target addr failed");
                        continue;
                    };

                    // 下面这一段逻辑把 target 转成了 ipv6，这样就同时兼容了 ipv4 和 ipv6
                    let Ok(mut target_addr) = target_addr.to_socket_addrs() else {
                        error!("Parse target addr error (Our proxy not supported domain)");
                        break;
                    };
                    let Some(mut target_addr) = target_addr.next() else {
                        error!("Parse target addr get next() error");
                        continue;
                    };

                    // 现在转成 ipv6，如果网络不支持，把上面 bind 的 ipv6 换成 ipv4，这里注释掉就行了
                    target_addr.set_ip(match target_addr.ip() {
                        std::net::IpAddr::V4(v4) => std::net::IpAddr::V6(v4.to_ipv6_mapped()),
                        v6 @ std::net::IpAddr::V6(_) => v6,
                    });
                    debug!("Write remote udp target addr: {:?}", target_addr);
                    debug!("Write remote udp data: {:?}", data);

                    if socket.send_to(&data, target_addr).await.is_err() {
                        warn!("Write remote udp error");
                        break;
                    }
                }
                RemoteMsg::ClosePort(_) => {
                    warn!("Remote udp close port");
                    break;
                }
                RemoteMsg::Data(_) => {
                    error!("Remote udp get tcp data");
                    break;
                }
            },
            None => {
                warn!("Read tunnel error");
                break;
            }
        }
    }

    running.store(false, Ordering::Relaxed);
    reader_tunnel.rx = None;
    info!("Write remote udp end");
    let msg = RemoteToServer::ClosePort(id).into();
    if reader_tunnel.send(msg).await.is_err() {
        error!("Write tunnel error 5");
    }
}
