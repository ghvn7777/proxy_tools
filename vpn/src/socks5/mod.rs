use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use socks5_stream::{parse_udp_request, Socks5Stream};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream, UdpSocket,
    },
};
use tracing::{debug, error, info, warn};

use crate::{
    util::TargetAddr, ClientConfig, ClientMsg, ServiceError, Socks5ToClientMsg, SocksMsg,
    TunnelReader, TunnelWriter, VpnError,
};

pub mod command;
pub mod socks5_stream;

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
            command::Socks5Command::TCPConnect => {
                self.tcp_connect(id, target_addr, write_port, read_port)
                    .await?;
            }
            command::Socks5Command::TCPBind => {
                error!("Tcp bind currently not supported: {:?}", cmd);
                return Ok(());
            }
            command::Socks5Command::UDPAssociate => {
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
        // The DST.ADDR and DST.PORT fields contain the address and port that
        // the client expects to use to send UDP datagrams on for the
        // association. The server MAY use this information to limit access
        // to the association.
        // @see Page 6, https://datatracker.ietf.org/doc/html/rfc1928.
        //
        // We do NOT limit the access from the client currently in this implementation.
        let _not_used = target_addr;

        // Listen with UDP6 socket, so the client can connect to it with either
        // IPv4 or IPv6.
        let socket = UdpSocket::bind("[::]:0").await?;

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
        // 这里和 tcp 代理不一样，因为 tcp 是只往 socket server 发数据，socks server 监听 0.0.0.0:0
        // 而 udp 代理要新建一个 udp socket 来接收代理数据
        info!("reply udp associate local addr: {:?}", socket.local_addr()?);
        if success {
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

        let r = udp_proxy_socks_read(&socket, &mut write_port);
        let w = udp_proxy_socks_write(&socket, &mut read_port);
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

        Ok(())
    }
}

async fn udp_proxy_socks_read(inbound: &UdpSocket, write_port: &mut TunnelWriter<ClientMsg>) {
    let mut buf = vec![0u8; 1024 * 4];
    loop {
        match inbound.recv_from(&mut buf).await {
            Ok((n, client_addr)) => {
                debug!("Socks5 server receive udp from {}", client_addr);
                let mut buf2 = Vec::with_capacity(n);
                buf2.extend_from_slice(&buf[0..n]);
                let Ok((frag, _target_addr, _data)) = parse_udp_request(&buf[..n]).await else {
                    error!("Parse UDP request error");
                    continue;
                };

                if frag != 0 {
                    // https://datatracker.ietf.org/doc/html/rfc1928
                    error!("Discard UDP frag packets silently.");
                    break; // 不支持分片，分片的 UDP 包还不如直接走 TCP
                }

                // if write_port
                //     .send(Socks5ToClientMsg::UdpData(client_addr, Box::new(buf2)).into())
                //     .await
                //     .is_err()
                // {
                //     warn!("Socks5 read send data error");
                //     break;
                // }
            }
            Err(e) => {
                warn!("Socks5 udp read error: {:?}", e);
                break;
            }
        }
    }

    if write_port
        .send(Socks5ToClientMsg::ClosePort(0).into())
        .await
        .is_err()
    {
        error!("Socks5 udp read send close port error");
    }
}

async fn udp_proxy_socks_write(
    _inbound: &UdpSocket,
    _read_port: &mut TunnelReader<ClientMsg, SocksMsg>,
) {
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
