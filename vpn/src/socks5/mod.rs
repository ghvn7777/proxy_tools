use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use socks5_stream::Socks5Stream;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};
use tracing::{debug, error, info, warn};

use crate::{
    ClientConfig, ClientMsg, ClientToSocks5Msg, ServiceError, Socks5ToClientMsg, TunnelReader,
    TunnelWriter, VpnError,
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
        self,
        mut write_port: TunnelWriter<ClientMsg>,
        mut read_port: TunnelReader<ClientMsg, ClientToSocks5Msg>,
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

        let mut stream = self.inner;
        stream.handshake(&self.config.auth_type).await?;

        let (cmd, target_addr) = stream.request(self.config.dns_resolve).await?;
        debug!("socks5 get target {:?}, cmd: {:?}", target_addr, cmd);

        if !(stream.check_command(&cmd).await?) {
            error!("Udp currently not supported: {:?}", cmd);
            return Ok(());
        }

        write_port
            .send(Socks5ToClientMsg::TcpConnect(id, target_addr).into())
            .await?;

        // 注意这里的 read_port 不会收到心跳数据，因为这个数据路径是：
        // server 返回给 client，client 通过 sender 发给 receivers，然后在 receivers 里面会过滤，
        // 只有特定的一些数据才会通过 port_map 发到 read_port
        // TODO: 如果数据类型容易混淆的话，可以用一个新的数据解构表示 report 的 tx
        let success = match read_port.read().await {
            Some(ClientToSocks5Msg::TcpConnectSuccess(id)) => {
                info!("Read tcp connect success: {}", id);
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

        if success {
            stream
                .send_reply(0, SocketAddr::new([127, 0, 0, 1].into(), 0))
                .await?;
            let (reader, writer) = stream.stream.into_split();
            let r = proxy_socks_read(reader, &mut write_port);
            let w = proxy_socks_write(writer, &mut read_port);
            // join!(r, w);
            tokio::select! {
                _ = r => {
                    info!("Socks5 proxy read end");
                }
                _ = w => {
                    info!("Socks5 proxy write end");
                }
            };
            info!("Socks5 proxy finished id: {}", read_port.get_id());
        } else {
            stream.send_reply(1, "0.0.0.0:0".parse().unwrap()).await?;
            write_port
                .send(Socks5ToClientMsg::ClosePort(id).into())
                .await?;
            return Err(
                ServiceError::TcpConnectError("Read port call read() error".to_string()).into(),
            );
        }

        Ok(())
    }
}

async fn proxy_socks_read(mut reader: OwnedReadHalf, write_port: &mut TunnelWriter<ClientMsg>) {
    let mut buf = vec![0u8; 1024 * 1000];
    let id = write_port.get_id();
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => {
                info!("Socks5 read 0 bytes, close port: {}", id);
                break;
            }
            Ok(n) => {
                let data = buf[..n].to_vec();
                info!("Socks5 read {} bytes", n);
                if write_port
                    .send(Socks5ToClientMsg::Data(id, data).into())
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

async fn proxy_socks_write(
    mut writer: OwnedWriteHalf,
    read_port: &mut TunnelReader<ClientMsg, ClientToSocks5Msg>,
) {
    loop {
        match read_port.read().await {
            Some(ClientToSocks5Msg::Data(id, data)) => {
                if id != read_port.get_id() {
                    warn!(
                        "Socks5 write get unexpected id: {} != {}",
                        id,
                        read_port.get_id()
                    );
                    break;
                }

                info!("Socks5 write {} bytes", data.len());
                if writer.write_all(&data).await.is_err() {
                    warn!("Socks5 write error, close port: {}", id);
                    break;
                }
            }
            Some(ClientToSocks5Msg::ClosePort(id)) => {
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

    let id = read_port.get_id();
    if read_port
        .send(Socks5ToClientMsg::ClosePort(id).into())
        .await
        .is_err()
    {
        error!("Socks5 write send close port error");
    }
}
