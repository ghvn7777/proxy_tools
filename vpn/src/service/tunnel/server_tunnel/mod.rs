use std::net::ToSocketAddrs;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream, UdpSocket,
    },
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    util::{TargetAddr, ToTargetAddr},
    RemoteMsg, RemoteToServer, ServerMsg, TunnelReader, TunnelWriter,
};

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

    tokio::select! {
        _ = r => {
            info!("Read remote tcp task end");
        }
        _ = w => {
            info!("Write remote tcp task end");
        }
    };
    // join!(w, r);
    info!("Server tunnel port tcp task end id: {}", id);
}

async fn read_remote_tcp(
    id: u32,
    mut stream: OwnedReadHalf,
    mut writer_tunnel: TunnelWriter<ServerMsg>,
) {
    let mut buf = vec![0u8; 1024 * 4];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => {
                info!("Tcp remote read 0");
                break;
            }

            Ok(n) => {
                let mut buf2 = Vec::with_capacity(n);
                buf2.extend_from_slice(&buf[0..n]);
                let msg = RemoteToServer::Data(id, Box::new(buf2)).into();
                info!("Read remote tcp data len: {:?}", n);
                if writer_tunnel.send(msg).await.is_err() {
                    error!("Write tunnel error");
                    break;
                }
            }

            Err(_) => {
                error!("Server Tcp Stream read error");
                break;
            }
        }
    }

    info!("Read remote tcp end");
    let msg = RemoteToServer::ClosePort(id).into();
    if writer_tunnel.send(msg).await.is_err() {
        error!("Write tunnel error");
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
        error!("Write tunnel error");
    }
}

pub async fn udp_tunnel_port_task(
    reader_tunnel: TunnelReader<ServerMsg, RemoteMsg>,
    mut writer_tunnel: TunnelWriter<ServerMsg>,
) {
    trace!("udp_runnel_port_task: start");
    let id = reader_tunnel.get_id();

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

    let _ = writer_tunnel
        .send(RemoteToServer::UdpAssociateSuccess(id).into())
        .await;

    let r = read_remote_udp(id, &socket, writer_tunnel);
    let w = write_remote_udp(id, &socket, reader_tunnel);

    tokio::select! {
        _ = r => {
            info!("Read remote udp task end");
        }
        _ = w => {
            info!("Write remote udp task end");
        }
    };
    // join!(w, r);
    info!("Server tunnel port task udp end id: {}", id);
}

async fn read_remote_udp(id: u32, socket: &UdpSocket, mut writer_tunnel: TunnelWriter<ServerMsg>) {
    let mut buf = vec![0u8; 1024 * 4];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((n, source)) => {
                let Ok(target_addr) = source.to_target_addr() else {
                    error!("Read remote udp target addr failed");
                    continue;
                };

                let mut buf2 = Vec::with_capacity(n);
                buf2.extend_from_slice(&buf[0..n]);
                let msg = RemoteToServer::UdpData(id, target_addr, Box::new(buf2)).into();
                info!("Read remote udp data len: {:?}", n);
                if writer_tunnel.send(msg).await.is_err() {
                    error!("Write tunnel error");
                    break;
                }
            }
            Err(_) => {
                error!("Server Udp Socket read error");
                break;
            }
        }
    }

    info!("Read remote udp end");
    let msg = RemoteToServer::ClosePort(0).into();
    if writer_tunnel.send(msg).await.is_err() {
        error!("Write tunnel error");
    }
}

async fn write_remote_udp(
    id: u32,
    socket: &UdpSocket,
    mut reader_tunnel: TunnelReader<ServerMsg, RemoteMsg>,
) {
    loop {
        match reader_tunnel.read().await {
            Some(msg) => match msg {
                RemoteMsg::UdpData(target_addr, data) => {
                    info!("Write remote udp data: {}, {:?}", target_addr, data.len());
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

                    target_addr.set_ip(match target_addr.ip() {
                        std::net::IpAddr::V4(v4) => std::net::IpAddr::V6(v4.to_ipv6_mapped()),
                        v6 @ std::net::IpAddr::V6(_) => v6,
                    });

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

    reader_tunnel.rx = None;
    info!("Write remote udp end");
    let msg = RemoteToServer::ClosePort(id).into();
    if reader_tunnel.send(msg).await.is_err() {
        error!("Write tunnel error");
    }
}
