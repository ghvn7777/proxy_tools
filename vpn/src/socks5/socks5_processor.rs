use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Result;
use async_std::{io, net::UdpSocket};
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    join, SinkExt, StreamExt,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    util::TargetAddr, ClientConfig, ClientMsg, ServiceError, Socks5ToClientMsg, SocksMsg,
    TunnelReader, TunnelWriter, VpnError,
};

use super::{new_udp_header, parse_udp_request, Socks5Command, Socks5Stream};

pub struct Socks5ServerStream<S = TcpStream> {
    inner: Socks5Stream<S>,
    config: Arc<ClientConfig>,
}

impl Socks5ServerStream<TcpStream> {
    pub fn new(stream: TcpStream, config: Arc<ClientConfig>) -> Self {
        Self {
            inner: Socks5Stream::new(stream),
            config,
        }
    }

    pub async fn process(
        mut self,
        write_port: TunnelWriter<ClientMsg>,
        read_port: TunnelReader<ClientMsg, SocksMsg>,
    ) -> Result<(), VpnError> {
        let id = write_port.get_id();
        if id != read_port.get_id() {
            warn!(
                "Write port id: {}, read port id: {}",
                id,
                read_port.get_id()
            );
            return Err(
                ServiceError::ChannelIdError(format!("{} != {}", id, read_port.get_id())).into(),
            );
        }

        let stream = &mut self.inner;
        stream.handshake(&self.config.auth_type).await?;

        let (cmd, target_addr) = stream.request(self.config.dns_resolve).await?;
        debug!("socks5 get target {:?}, cmd: {:?}", target_addr, cmd);

        match cmd {
            Socks5Command::TCPConnect => {
                self.tcp_connect(id, target_addr, write_port, read_port)
                    .await?;
            }
            Socks5Command::TCPBind => {
                error!("Tcp bind currently not supported: {:?}", cmd);
                return Ok(());
            }
            Socks5Command::UDPAssociate => {
                self.udp_associate(id, target_addr, write_port, read_port)
                    .await?;
                return Ok(());
            }
        }

        Ok(())
    }

    async fn tcp_connect(
        self,
        id: u32,
        target_addr: TargetAddr,
        mut write_port: TunnelWriter<ClientMsg>,
        mut read_port: TunnelReader<ClientMsg, SocksMsg>,
    ) -> Result<(), VpnError> {
        let mut stream = self.inner;
        write_port
            .send(Socks5ToClientMsg::TcpConnect(id, target_addr).into())
            .await?;

        // The supplied BND.ADDR is often different from the IP address that the client uses to reach the SOCKS server,
        // since such servers are often multi-homed.
        // It is expected that the SOCKS server will use DST.ADDR and DST.PORT,
        // and the client-side source address and port in evaluating the CONNECT request.
        // Refer: https://datatracker.ietf.org/doc/html/rfc1928
        // 按照上面的描述，TCP 连接这里应该返回的是真正的服务端的地址，而不是客户端的地址
        // 虽然对于连接没有影响，但是也可以让客户端评估连接质量，还是按照标准来了
        let bind_addr: Option<SocketAddr> = match read_port.read().await {
            Some(SocksMsg::TcpConnectSuccess(id, bind_addr)) => {
                info!(
                    "Read tcp connect success: id = {}, bind_addr = {}",
                    id, bind_addr
                );
                match bind_addr {
                    TargetAddr::Ip(addr) => Some(addr),
                    TargetAddr::Domain(domain, port) => {
                        error!("bind addr is domain: {}:{}", domain, port);
                        None
                    }
                }
            }
            Some(msg) => {
                warn!("Read port get unexpected msg: {:?}", msg);
                None
            }
            None => {
                warn!("Read port get None");
                None
            }
        };

        let success = if let Some(bind_addr) = bind_addr {
            if stream.send_reply(0, bind_addr).await.is_err() {
                error!("Socks5 proxy send reply error");
                write_port
                    .send(Socks5ToClientMsg::ClosePort(id).into())
                    .await?;
                false
            } else {
                true
            }
        } else {
            let _ = stream.send_reply(1, "0.0.0.0:0".parse().unwrap()).await;
            write_port
                .send(Socks5ToClientMsg::ClosePort(id).into())
                .await?;
            false
        };

        if success {
            debug!("socks5 tcp connect success");
            let (reader, writer) = stream.stream.into_split();
            let r = tcp_proxy_socks_read(reader, &mut write_port);
            let w = tcp_proxy_socks_write(writer, &mut read_port);
            // join!(r, w);
            tokio::select! {
                _ = r => {
                    info!("Socks5 proxy read end");
                }
                _ = w => {
                    info!("Socks5 proxy write end");
                }
            };
            info!("Socks5 tcp proxy finished id: {}", read_port.get_id());
        }

        Ok(())
    }

    async fn udp_associate(
        self,
        id: u32,
        target_addr: TargetAddr,
        mut write_port: TunnelWriter<ClientMsg>,
        mut read_port: TunnelReader<ClientMsg, SocksMsg>,
    ) -> Result<(), VpnError> {
        trace!("Socks5 udp associate: id = {}", id);
        // The DST.ADDR and DST.PORT fields contain the address and port that
        // the client expects to use to send UDP datagrams on for the
        // association. The server MAY use this information to limit access
        // to the association.
        // @see Page 6, https://datatracker.ietf.org/doc/html/rfc1928.
        //
        // We do NOT limit the access from the client currently in this implementation.
        let _not_used = target_addr;

        // Listen with UDP6 socket, so the client can connect to it with either IPv4 or IPv6.
        // 先注释掉，因为没想好返回给客户端什么地址，还有就是有的客户端不支持 ipv6
        // let socket = UdpSocket::bind("[::]:0").await?;

        // 因为下面直接用了 socket 地址返回给客户端，所以这里不能是 0.0.0.0，但是这么写只能做本地代理
        // 地址后续可以考虑用局域网地址，可以代理局域网 udp 流量
        let socket = UdpSocket::bind("127.0.0.1:0").await?;

        let mut stream = self.inner;
        write_port
            .send(Socks5ToClientMsg::UdpAssociate(id).into())
            .await?;

        let success = match read_port.read().await {
            Some(SocksMsg::UdpAssociateSuccess(id)) => {
                info!("Read udp associate success: id = {}", id);
                true
            }
            Some(msg) => {
                warn!("Read port get unexpected msg: {:?}", msg);
                false
            }
            None => {
                warn!("Read port get None");
                false
            }
        };

        // TODO: 如果 local_addr 是 127.0.0.1:x 的话
        // 那应该只能代理在本地 udp 流量，后续可以考虑用局域网地址，可以代理局域网 udp 流量
        // 这里和 tcp 代理不一样，因为 tcp 是只往 socket server 固定端口发数据，socks server 监听 0.0.0.0
        // 而 udp 代理每次要新建一个 udp socket 来接收代理数据
        if success {
            info!("reply udp associate local addr: {:?}", socket.local_addr()?);
            if stream.send_reply(0, socket.local_addr()?).await.is_err() {
                error!("Socks5 proxy send reply error");
                write_port
                    .send(Socks5ToClientMsg::ClosePort(id).into())
                    .await?;
                return Ok(());
            }
        } else {
            let _ = stream.send_reply(1, "0.0.0.0:0".parse().unwrap()).await;
            write_port
                .send(Socks5ToClientMsg::ClosePort(id).into())
                .await?;
            return Ok(());
        }

        // 因为 udp_proxy_socks_read 里面用的 async_std 的 timeout，没有阻塞到 future 里，所以无法通过 tokio::select! 退出
        // 我们在这里用一个 atomic bool 来控制退出
        // 服务器还有个超时时间控制退出，如果超时时间到了，就会发送 close port 指令退出
        // 客户端这里就不做了，因为有个和 client 的 TCP 连接，不用超时作为退出条件，用 TCP 连接断开作为退出条件
        let running = AtomicBool::new(true);
        // since the UDP associate is established, we can know the client addr,
        // so we can send the client addr to the writer through channel when we receive the first packet
        let (client_addr_tx, client_addr_rx) = mpsc::channel(1);
        let r = udp_proxy_socks_read(&running, &socket, &mut write_port, client_addr_tx);
        let w = udp_proxy_socks_write(&running, &socket, &mut read_port, client_addr_rx);
        // 协议要求 tcp 保持连接，不然认为 udp 代理需要断开
        let h = tcp_connection_holder(&running, stream.stream);
        // 因为有 running 控制，所以这里一定会全部退出，可以直接用 join! 保证 close port 指令一定会发出去通知到服务器
        // 因为这里多了个 tcp_connection_holder 任务，它没有办法发送 close port 指令，所以这里要用 join 保证。
        // 其它时候只有两个任务，两个任务内部在退出时候会发送 close port 指令，所以不用 join
        join!(r, w, h);
        // tokio::select! {
        //     _ = r => {
        //         info!("Socks5 udp proxy read end");
        //     }
        //     _ = w => {
        //         info!("Socks5 udp proxy write end");
        //     }
        //     _ = h => {

        //         info!("Socks5 udp proxy holder end");
        //     }
        // };
        info!("Socks5 udp proxy finished id: {}", read_port.get_id());

        Ok(())
    }
}

async fn udp_proxy_socks_read(
    running: &AtomicBool,
    socket: &UdpSocket,
    write_port: &mut TunnelWriter<ClientMsg>,
    mut client_addr_tx: Sender<SocketAddr>,
) {
    let id = write_port.get_id();
    // 这里的 buffer 大了一些，因为 udp 是不可靠的，如果数据包太大，buffer 小了可能会导致数据丢失
    let mut buf = vec![0u8; 1024 * 10];
    let mut send_client_add_flag = false;
    loop {
        if !running.load(Ordering::Relaxed) {
            info!("Running is false Socks5 udp server read end");
            break;
        }
        // 注意这里设置了超时，如果没有数据，就会返回超时错误，不会一直阻塞，因为测试时候发现一直阻塞，反而收不到数据
        // 有超时时间会收到数据，还不清楚原因
        // 由于这里 io 是 async_std 的，为了兼容好一点，socket 也用了 async_std 的 UdpSocket
        let res = io::timeout(Duration::from_millis(100), socket.recv_from(&mut buf)).await;
        match res {
            Ok((n, client_addr)) => {
                debug!("Socks5 udp server receive {} bytes from {}", n, client_addr);
                if !send_client_add_flag {
                    debug!("Sending client addr: {}", client_addr);
                    if client_addr_tx.send(client_addr).await.is_err() {
                        error!("Send client addr error");
                        break;
                    }
                    send_client_add_flag = true;
                }

                // 这里参数很有意思， 传进去的是 &[u8]，也就是这段数据不能变，接收参数是 mut req: &[u8],
                // 这里参数细节是，参数只要保证数据是不可变的就行，也就是 &[u8] 是个不可变引用，
                // 但是 req 本身可以变，相当于指针可以变，但是指针指向的数据不能变，
                // 也就是 req = req2 可以，req[0] = 1 不行
                // 另外说明一下，里面参数声明为 mut req: &[u8]，是因为 read_exac! 宏要用 req 模拟 stream 读取数据
                // 不需要改变数据，比如说读取完 2 个字节，只需要改变 req 的指针就行，所以这里是 &mut req
                let Ok((frag, target_addr, data)) = parse_udp_request(&buf[..n]).await else {
                    error!("Parse UDP request error");
                    continue;
                };

                if frag != 0 {
                    // https://datatracker.ietf.org/doc/html/rfc1928
                    error!("Discard UDP frag packets silently.");
                    break; // 不支持分片，分片的 UDP 包还不如直接走 TCP
                }

                debug!("UDP target addr: {}", target_addr);
                debug!("UDP data len: {}", data.len());

                // TODO: 还有别人的代码一般不支持 domain，我这里没检查直接发到服务器解析，不知道他们不支持是不是有什么考虑
                let mut buf2 = Vec::with_capacity(data.len());
                buf2.extend_from_slice(&data[0..data.len()]);
                if write_port
                    .send(Socks5ToClientMsg::UdpData(id, target_addr, Box::new(buf2)).into())
                    .await
                    .is_err()
                {
                    warn!("Socks5 read send data error");
                    break;
                }
            }
            Err(e) => {
                warn!("Socks5 udp read error(may be timeout): {:?}", e);
                // break;
            }
        }
    }

    running.store(false, Ordering::Relaxed);
    if write_port
        .send(Socks5ToClientMsg::ClosePort(id).into())
        .await
        .is_err()
    {
        error!("Socks5 udp read send close port error");
    }
}

async fn udp_proxy_socks_write(
    running: &AtomicBool,
    inbound: &UdpSocket,
    read_port: &mut TunnelReader<ClientMsg, SocksMsg>,
    mut client_addr_rx: Receiver<SocketAddr>,
) {
    let id = read_port.get_id();
    let client_addr = match client_addr_rx.next().await {
        Some(addr) => addr,
        None => {
            error!("Client addr is none");
            read_port.rx = None;
            if read_port
                .send(Socks5ToClientMsg::ClosePort(id).into())
                .await
                .is_err()
            {
                error!("Socks5 udp write send close port error");
            }
            return;
        }
    };

    info!(
        "Udp proxy socks id: {} writer get client addr: {}",
        id, client_addr
    );

    // 这里如果关闭注释，进行 connect，后面需要用 send_to 函数指定地址发送，否则 connect 后直接用 send 就可以了
    // 感觉这样更好，不用每次都指定地址
    // if inbound.connect(client_addr).await.is_err() {
    //     error!("Udp proxy socks writer connect client addr error");
    //     read_port.rx = None;
    //     if read_port
    //         .send(Socks5ToClientMsg::ClosePort(id).into())
    //         .await
    //         .is_err()
    //     {
    //         error!("Socks5 udp write send close port error");
    //     }
    //     return;
    // }

    loop {
        if !running.load(Ordering::Relaxed) {
            info!("Running is false Socks5 udp server write end");
            break;
        }

        match read_port.read().await {
            Some(SocksMsg::UdpData(target_addr, data)) => {
                // 这里我们支持域名格式
                debug!("Socks5 udp write {} bytes to {}", data.len(), client_addr);
                let Ok(mut udp_data) = new_udp_header(target_addr) else {
                    error!("New UDP header error");
                    break;
                };

                udp_data.extend_from_slice(&data[..data.len()]);
                // let _ = inbound.send_to(&data, client_addr).await;
                if inbound.send_to(&udp_data, client_addr).await.is_err() {
                    warn!("Socks5 udp write error");
                    break;
                }
            }
            Some(SocksMsg::ClosePort(id)) => {
                info!("Socks5 udp write close port id: {}", id);
                break;
            }
            Some(msg) => {
                warn!("Socks5 udp write get unexpected msg: {:?}", msg);
                break;
            }
            None => {
                warn!("Socks5 udp write get None");
                break;
            }
        }
    }

    running.store(false, Ordering::Relaxed);
    read_port.rx = None;
    if read_port
        .send(Socks5ToClientMsg::ClosePort(id).into())
        .await
        .is_err()
    {
        error!("Socks5 udp write send close port error");
    }
}

async fn tcp_connection_holder(running: &AtomicBool, mut stream: TcpStream) {
    let mut buf = vec![0u8; 1024];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => {
                info!("Socks5 tcp connection holder read 0 bytes");
                break;
            }
            Ok(n) => {
                info!("Socks5 tcp connection holder read {} bytes", n);
            }
            Err(e) => {
                warn!("Socks5 tcp connection holder read error: {:?}", e);
                break;
            }
        }
    }

    running.store(false, Ordering::Relaxed);
    info!("Socks5 tcp connection holder end, running set false");
}

async fn tcp_proxy_socks_read(mut reader: OwnedReadHalf, write_port: &mut TunnelWriter<ClientMsg>) {
    let mut buf = vec![0u8; 1024 * 4];
    let id = write_port.get_id();
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => {
                info!("Socks5 read 0 bytes, close port: {}", id);
                break;
            }
            Ok(n) => {
                let mut buf2 = Vec::with_capacity(n);
                buf2.extend_from_slice(&buf[0..n]);
                info!("Socks5 read {} bytes", n);
                if write_port
                    .send(Socks5ToClientMsg::Data(id, Box::new(buf2)).into())
                    .await
                    .is_err()
                {
                    warn!("Socks5 read send data error, close port: {}", id);
                    break;
                }
            }
            Err(e) => {
                warn!("Socks5 read error: {:?}", e);
                break;
            }
        }
    }
    if write_port
        .send(Socks5ToClientMsg::ClosePort(id).into())
        .await
        .is_err()
    {
        error!("Socks5 read send close port error, close port: {}", id);
    }
}

async fn tcp_proxy_socks_write(
    mut writer: OwnedWriteHalf,
    read_port: &mut TunnelReader<ClientMsg, SocksMsg>,
) {
    let id = read_port.get_id();
    loop {
        match read_port.read().await {
            Some(SocksMsg::Data(data)) => {
                info!("Socks5 write {} bytes", data.len());
                if writer.write_all(&data).await.is_err() {
                    warn!("Socks5 write error, close port: {}", id);
                    break;
                }
            }
            Some(SocksMsg::ClosePort(id)) => {
                info!("Socks5 write close port id: {}", id);
                break;
            }
            Some(msg) => {
                warn!("Socks5 write get unexpected msg: {:?}", msg);
                break;
            }
            None => {
                warn!("Socks5 write get None");
                break;
            }
        }
    }

    read_port.rx = None;
    if read_port
        .send(Socks5ToClientMsg::ClosePort(id).into())
        .await
        .is_err()
    {
        error!("Socks5 write send close port error");
    }
}
