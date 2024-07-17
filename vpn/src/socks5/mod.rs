use std::sync::Arc;

use anyhow::Result;
use socks5_stream::Socks5Stream;
use tokio::net::TcpStream;
use tracing::{debug, warn};

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
        read_port: TunnelReader<ClientMsg, ClientToSocks5Msg>,
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
            .send(Socks5ToClientMsg::TcpConnect(id, target_addr).into())
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
        // write_port.send(Socks5ToClientMsg::ClosePort(id).into()).await?;
        Ok(())
    }
}
