use std::sync::Arc;

use futures::{SinkExt, Stream, StreamExt};
use tokio::io::AsyncWrite;
use tracing::{error, info};

use crate::{
    pb::CommandRequest, ChannelMap, ProstWriteStream, ServiceError, Socks5ToClientMsg, VpnError,
};

// TODO: read 和 write stream 里的 vpn 类型给反了
pub struct VpnProstWriteStream<S> {
    inner: ProstWriteStream<S, CommandRequest>,
}

impl<S> VpnProstWriteStream<S>
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
        channel_map: Arc<ChannelMap>,
    ) -> Result<(), VpnError>
    where
        T: Stream<Item = Socks5ToClientMsg> + Unpin,
    {
        loop {
            match msg_stream.next().await {
                Some(msg) => self.execute(msg, channel_map.clone()).await?,
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
        channel_map: Arc<ChannelMap>,
    ) -> Result<(), VpnError> {
        match msg {
            Socks5ToClientMsg::InitChannel(id, tx) => {
                if channel_map.insert(id, tx).is_some() {
                    error!("Init channel failed, channel id already exists");
                    return Err(ServiceError::ChannelIdExists(id).into());
                }
                info!("Init channel: {:?}", channel_map);
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
