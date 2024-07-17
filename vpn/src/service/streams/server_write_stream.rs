use std::collections::HashMap;
use std::time::Duration;

use futures::channel::mpsc::{channel, Receiver};
use futures::stream::SelectAll;
use futures::SinkExt;
use tokio::io::AsyncWrite;
use tokio_stream::StreamExt;
use tracing::{error, info};

use crate::{interval, TunnelReader, TunnelWriter, HEARTBEAT_INTERVAL_MS};
use crate::{pb::CommandResponse, util::SubSenders, ProstWriteStream, ServerMsg, VpnError};

pub type Receivers<T> = SelectAll<Receiver<T>>;

pub struct VpnServerProstWriteStream<S> {
    inner: ProstWriteStream<S, CommandResponse>,
}

impl<S> VpnServerProstWriteStream<S>
where
    S: AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: ProstWriteStream::new(stream),
        }
    }

    pub async fn send(&mut self, msg: &CommandResponse) -> Result<(), VpnError> {
        self.inner.send(msg).await
    }

    pub async fn process(
        &mut self,
        mut subsenders: SubSenders<ServerMsg>,
        receivers: Receivers<ServerMsg>,
    ) -> Result<(), VpnError> {
        let mut server_port_map = HashMap::new();
        let duration = Duration::from_millis(HEARTBEAT_INTERVAL_MS);
        let timer_stream = interval(duration, ServerMsg::Heartbeat);
        let mut msg_stream = timer_stream.merge(receivers);

        loop {
            match msg_stream.next().await {
                Some(msg) => {
                    match msg {
                        ServerMsg::Heartbeat => {
                            info!("Server heartbeat");
                        }
                        ServerMsg::ClosePort(id) => {
                            info!("Close port: {}", id);
                            todo!("server close port todo");
                            // subsenders.close_port(id).await?;
                        }
                        ServerMsg::TcpConnect(id, _target_addr) => {
                            let (tx, rx) = channel::<ServerMsg>(1000);
                            server_port_map.insert(id, tx);
                            let sender = subsenders.get_one_sender();
                            let _reader_remote = TunnelReader {
                                id,
                                tx: sender.clone(),
                                rx: Some(rx),
                            };
                            let _writer_remote = TunnelWriter {
                                id,
                                tx: sender.clone(),
                            };

                            tokio::spawn(async move {
                                // tunnel_port_task(reader_remote, writer_remote).await;
                            });
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
}
