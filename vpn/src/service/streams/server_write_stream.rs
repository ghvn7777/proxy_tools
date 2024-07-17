use std::collections::HashMap;
use std::time::{Duration, Instant};

use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::stream::SelectAll;
use futures::SinkExt;
use tokio::io::AsyncWrite;
use tokio_stream::StreamExt;
use tracing::{error, info, trace};

use crate::{
    interval, tunnel_port_task, RemoteMsg, RemoteToServer, ServerToRemote, TunnelReader,
    TunnelWriter, ALIVE_TIMEOUT_TIME_MS, HEARTBEAT_INTERVAL_MS,
};
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
        let alive_time = Instant::now();
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
                            // info!("alive time {:?}, now {:?}", alive_time, Instant::now());
                            if Instant::now() - alive_time
                                > Duration::from_millis(ALIVE_TIMEOUT_TIME_MS)
                            {
                                error!("Server heartbeat timeout");
                                break;
                            }
                        }
                        ServerMsg::ServerToRemote(msg) => {
                            self.process_server_to_remote(
                                msg,
                                &mut server_port_map,
                                &mut subsenders,
                            )
                            .await;
                        }
                        ServerMsg::RemoteToServer(msg) => {
                            self.process_remote_to_server(
                                msg,
                                &mut server_port_map,
                                &mut subsenders,
                            )
                            .await;
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

    pub async fn process_server_to_remote(
        &mut self,
        msg: ServerToRemote,
        server_port_map: &mut HashMap<u32, Sender<RemoteMsg>>,
        subsenders: &mut SubSenders<ServerMsg>,
    ) {
        trace!("Server to remote: {:?}", msg);
        match msg {
            ServerToRemote::ClosePort(id) => {
                info!("SClose port: {}", id);
                server_port_map.remove(&id);
            }
            ServerToRemote::TcpConnect(id, target_addr) => {
                let (tx, rx) = channel(1000);
                server_port_map.insert(id, tx);
                let sender = subsenders.get_one_sender();
                let reader_remote = TunnelReader {
                    id,
                    tx: sender.clone(),
                    rx: Some(rx),
                };
                let writer_remote = TunnelWriter {
                    id,
                    tx: sender.clone(),
                };

                tokio::spawn(async move {
                    tunnel_port_task(id, target_addr, reader_remote, writer_remote).await;
                });
            }
            ServerToRemote::Data(id, data) => {
                info!("Server get data: {:?}", data);
                if let Some(tx) = server_port_map.get_mut(&id) {
                    if tx.send(RemoteMsg::Data(data)).await.is_err() {
                        info!("Server send data failed");
                        server_port_map.remove(&id);
                    }
                }
            }
        }
    }

    pub async fn process_remote_to_server(
        &mut self,
        msg: RemoteToServer,
        server_port_map: &mut HashMap<u32, Sender<RemoteMsg>>,
        _subsenders: &mut SubSenders<ServerMsg>,
    ) {
        match msg {
            RemoteToServer::TcpConnectSuccess(id) => {
                info!("Remote connect success: {}", id);
                if self
                    .send(&CommandResponse::new_tcp_connect_success(id))
                    .await
                    .is_err()
                {
                    info!("Server send tcp connect success failed");
                    server_port_map.remove(&id);
                }
            }
            RemoteToServer::TcpConnectFailed(id) => {
                info!("Remote connect failed: {}", id);
                if self
                    .send(&CommandResponse::new_tcp_connect_failed(id))
                    .await
                    .is_err()
                {
                    info!("Server send tcp connect failed failed");
                    server_port_map.remove(&id);
                }
            }
            RemoteToServer::Data(id, data) => {
                info!("Remote get data: {:?}", data);
                let msg = CommandResponse::new_data(id, data);
                if self.send(&msg).await.is_err() {
                    error!("Server send data failed");
                    server_port_map.remove(&id);
                }
            }
            RemoteToServer::ClosePort(id) => {
                info!("Remote close port: {}", id);
                if self
                    .send(&CommandResponse::new_close_port(id))
                    .await
                    .is_err()
                {
                    info!("Server send close port failed");
                    server_port_map.remove(&id);
                }
            }
        }
    }
}
