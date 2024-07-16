use std::sync::Arc;

use anyhow::Result;
use socks5_stream::Socks5Stream;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info, warn};

use crate::{
    ClientConfig, ServiceError, Socks5ToClientMsg, Tunnel, TunnelReadPort, TunnelWritePort,
    VpnError,
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
        let (write_port, read_port) = tunnel.generate().await?;
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
        mut write_port: TunnelWritePort,
        read_port: TunnelReadPort,
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

        if !(stream.check_command(cmd).await?) {
            warn!("Udp currently not supported");
            return Ok(());
        }

        write_port
            .send(Socks5ToClientMsg::TcpConnect(id, target_addr))
            .await?;
        // todo: wait read_port get connect success

        // if res.status() == Status::Success {
        //     // server connect target addr success
        //     stream
        //         .send_reply(0, SocketAddr::new([127, 0, 0, 1].into(), 0))
        //         .await?;
        // } else {
        //     // server connect target addr failed
        //     stream.send_reply(1, "0.0.0.0:0".parse().unwrap()).await?;
        //     return Ok(());
        // }

        Ok(())
    }
}
