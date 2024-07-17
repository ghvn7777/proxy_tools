use std::sync::Arc;

use futures::{SinkExt, Stream, StreamExt};
use tokio::io::AsyncWrite;
use tracing::{error, info};

use crate::{
    pb::CommandRequest, ClientPortMap, ProstWriteStream, ServiceError, Socks5ToClientMsg, VpnError,
};

pub struct VpnClientProstWriteStream<S> {
    inner: ProstWriteStream<S, CommandRequest>,
}

impl<S> VpnClientProstWriteStream<S>
where
    S: AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: ProstWriteStream::new(stream),
        }
    }

    pub async fn send(&mut self, msg: &CommandRequest) -> Result<(), VpnError> {
        self.inner.send(msg).await
    }

    pub async fn process<T>(
        &mut self,
        mut msg_stream: T,
        port_map: Arc<ClientPortMap>,
    ) -> Result<(), VpnError>
    where
        T: Stream<Item = Socks5ToClientMsg> + Unpin,
    {
        loop {
            match msg_stream.next().await {
                Some(msg) => self.execute(msg, port_map.clone()).await?,
                None => {
                    error!("Tunnel get none message, stop processing...");
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn execute(
        &mut self,
        msg: Socks5ToClientMsg,
        port_map: Arc<ClientPortMap>,
    ) -> Result<(), VpnError> {
        match msg {
            Socks5ToClientMsg::InitChannel(id, tx) => {
                if port_map.insert(id, tx).is_some() {
                    error!("Init channel failed, channel id already exists");
                    return Err(ServiceError::ChannelIdExists(id).into());
                }
                info!("Init channel: {:?}", port_map);
            }
            Socks5ToClientMsg::ClosePort(id) => {
                port_map.remove(&id);
                info!("Close port: {:?}", port_map);
                let msg = CommandRequest::new_close_port(id);
                self.send(&msg).await?;
            }
            Socks5ToClientMsg::TcpConnect(id, addr) => {
                info!("TcpConnect: {} -- {:?}", id, addr);
                let msg = CommandRequest::new_tcp_connect(id, addr);
                self.send(&msg).await?;
            }
            Socks5ToClientMsg::Heartbeat => {
                info!("Client WriteStream receive heartbeat");
            }
        }

        Ok(())
    }
}
