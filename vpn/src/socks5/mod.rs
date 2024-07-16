use std::sync::Arc;

use anyhow::Result;
use socks5_stream::Socks5Stream;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info, warn};

use crate::{
    tunnel::{Tunnel, TunnelReadPort, TunnelWritePort},
    ClientConfig, VpnError,
};

pub mod command;
pub mod socks5_stream;

pub struct Socks5ServerStream<S = TcpStream> {
    inner: Socks5Stream<S>,
    config: Arc<ClientConfig>,
}

pub async fn proxy_tunnels(
    mut tunnels: Vec<Tunnel>,
    config: Arc<ClientConfig>,
) -> Result<(), VpnError> {
    let socket_addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(socket_addr).await?;
    info!("Socks5 server listening on {}", listener.local_addr()?);

    let mut index = 0;

    loop {
        let socks5_config = config.clone();
        let (stream, addr) = listener.accept().await?;
        info!("Socks5 client {:?} connected", addr);

        let tunnel: &mut Tunnel = tunnels.get_mut(index).expect("Get tunnel failed");
        let (write_port, read_port) = tunnel.open_port().await;

        tokio::spawn(async move {
            let stream = Socks5ServerStream::new(stream, socks5_config);
            stream
                .process(write_port, read_port)
                .await
                .expect("proxy tunnel failed");
        });

        index = (index + 1) % tunnels.len();
    }
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
        _write_port: TunnelWritePort,
        _read_port: TunnelReadPort,
    ) -> Result<(), VpnError> {
        let mut stream = self.inner;
        stream.handshake(&self.config.auth_type).await?;
        let (cmd, _target_addr) = stream.request(self.config.dns_resolve).await?;
        debug!("request good");

        // TODO: 这里没有执行任何命令，后面想一下架构
        if !(stream.execute_command(cmd).await?) {
            warn!("Udp currently not supported");
            return Ok(());
        }

        Ok(())
    }
}
