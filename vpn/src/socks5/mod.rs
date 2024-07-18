use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use futures::join;
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
        self,
        mut write_port: TunnelWriter<ClientMsg>,
        mut read_port: TunnelReader<ClientMsg, SocksMsg>,
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
            debug!("socks5 connect success");
            let (reader, writer) = stream.stream.into_split();
            let r = proxy_socks_read(reader, &mut write_port);
            let w = proxy_socks_write(writer, &mut read_port);
            join!(r, w);
            // tokio::select! {
            //     _ = r => {
            //         info!("Socks5 proxy read end");
            //     }
            //     _ = w => {
            //         info!("Socks5 proxy write end");
            //     }
            // };
            info!("Socks5 proxy finished id: {}", read_port.get_id());
        }

        Ok(())
    }
}

async fn proxy_socks_read(mut reader: OwnedReadHalf, write_port: &mut TunnelWriter<ClientMsg>) {
    let mut buf = vec![0u8; 1024];
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

async fn proxy_socks_write(
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
