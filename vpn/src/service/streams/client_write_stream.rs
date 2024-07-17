use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use futures::{SinkExt, Stream, StreamExt};
use tokio::io::AsyncWrite;
use tracing::{error, info, trace};

use crate::{
    pb::CommandRequest, ClientMsg, ClientPortMap, ProstWriteStream, ServiceError,
    Socks5ToClientMsg, VpnError, ALIVE_TIMEOUT_TIME_MS,
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
        T: Stream<Item = ClientMsg> + Unpin,
    {
        let alive_time = Instant::now();
        loop {
            match msg_stream.next().await {
                Some(msg) => {
                    match msg {
                        ClientMsg::Heartbeat => {
                            info!("Client WriteStream receive heartbeat");
                            if Instant::now() - alive_time
                                > Duration::from_millis(ALIVE_TIMEOUT_TIME_MS)
                            {
                                // TODO: shutdown stream
                                error!("Client heartbeat timeout");
                                break;
                            }
                        }
                        ClientMsg::Socks5ToClient(msg) => {
                            self.process_socks5_to_client(msg, &port_map).await?
                        }
                        ClientMsg::ClientToSocks5(_) => {
                            todo!("ClientToSocks5 is not supported in ClientWriteStream")
                        }
                    }
                }
                None => {
                    error!("Tunnel get none message, stop processing...");
                    break;
                }
            }
        }

        Ok(())
    }

    /*
                       Socks5ToClientMsg::TcpConnect(id, addr) => {
                           info!("TcpConnect: {} -- {:?}", id, addr);
                           let msg = CommandRequest::new_tcp_connect(id, addr);
                           self.send(&msg).await?;
                       }

    */
    pub async fn process_socks5_to_client(
        &mut self,
        msg: Socks5ToClientMsg,
        port_map: &Arc<ClientPortMap>,
    ) -> Result<(), VpnError> {
        trace!("process_socks5_to_client: {:?}", msg);
        match msg {
            Socks5ToClientMsg::InitChannel(id, tx) => {
                if port_map.insert(id, tx).is_some() {
                    error!("Init channel failed, channel id already exists");
                    return Err(ServiceError::ChannelIdExists(id).into());
                }
                info!("Init channel: {:?}", port_map);
            }
            Socks5ToClientMsg::ClosePort(id) => {
                info!("Close port {}: {:?}", id, port_map);
                if port_map.remove(&id).is_none() {
                    error!("Port id not found: {}", id);
                }
                let msg = CommandRequest::new_close_port(id);
                self.send(&msg).await?;
            }
            Socks5ToClientMsg::TcpConnect(id, addr) => {
                info!("TcpConnect: {} -- {:?}", id, addr);
                let msg = CommandRequest::new_tcp_connect(id, addr);
                self.send(&msg).await?;
            }
        }
        Ok(())
    }
}
