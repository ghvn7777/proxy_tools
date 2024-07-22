use std::collections::HashMap;
use std::mem;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::stream::SelectAll;
use futures::SinkExt;
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;
use tonic::async_trait;
use tracing::{debug, error, info, trace};

use crate::{
    interval, tcp_tunnel_port_task, udp_tunnel_port_task, DataCrypt, ProstWriteStream, RemoteMsg,
    RemoteToServer, ServerToRemote, TunnelReader, TunnelWriter, WriteStream, ALIVE_TIMEOUT_TIME_MS,
    HEARTBEAT_INTERVAL_MS,
};
use crate::{pb::CommandResponse, util::SubSenders, ServerMsg, VpnError};

use super::Processor;

pub type Receivers<T> = SelectAll<Receiver<T>>;

pub struct ServerWriteProcessor<T> {
    inner: ProstWriteStream<CommandResponse, T>,
    subsenders: SubSenders<ServerMsg>,
    receivers: Option<Receivers<ServerMsg>>,
}

impl<T> ServerWriteProcessor<T> {
    pub fn new(
        stream: T,
        crypt: Arc<Box<dyn DataCrypt>>,
        subsenders: SubSenders<ServerMsg>,
        receivers: Receivers<ServerMsg>,
    ) -> Self {
        Self {
            inner: ProstWriteStream::new(stream, crypt),
            subsenders,
            receivers: Some(receivers),
        }
    }
}

#[async_trait]
impl<T> Processor for ServerWriteProcessor<T>
where
    T: AsyncWriteExt + Unpin + Send + Sync + 'static,
{
    async fn process(&mut self) -> Result<(), VpnError> {
        let mut alive_time = Instant::now();
        let mut server_port_map = HashMap::new();
        let duration = Duration::from_millis(HEARTBEAT_INTERVAL_MS);
        let timer_stream = interval(duration, ServerMsg::Heartbeat);
        if self.receivers.is_none() {
            error!("Server WriteStream receivers is none");
            return Ok(());
        }

        let receivers = mem::take(&mut self.receivers).unwrap();
        let mut msg_stream = timer_stream.merge(receivers);

        loop {
            match msg_stream.next().await {
                Some(ServerMsg::Heartbeat) => {
                    // info!("Server heartbeat");
                    // info!("alive time {:?}, now {:?}", alive_time, Instant::now());
                    if Instant::now() - alive_time > Duration::from_millis(ALIVE_TIMEOUT_TIME_MS) {
                        error!("Server heartbeat timeout");
                        break;
                    }
                }
                Some(ServerMsg::ServerToRemote(msg)) => {
                    alive_time = Instant::now();
                    self.process_server_to_remote(msg, &mut server_port_map)
                        .await;
                }
                Some(ServerMsg::RemoteToServer(msg)) => {
                    alive_time = Instant::now();
                    self.process_remote_to_server(msg, &mut server_port_map)
                        .await;
                }
                None => {
                    error!("Tunnel get none message, stop processing...");
                    break;
                }
            }
        }
        error!("Server WriteStream end");

        Ok(())
    }
}

impl<T> ServerWriteProcessor<T>
where
    T: AsyncWriteExt + Unpin + Send + Sync + 'static,
{
    pub async fn process_server_to_remote(
        &mut self,
        msg: ServerToRemote,
        server_port_map: &mut HashMap<u32, Sender<RemoteMsg>>,
    ) {
        trace!("Server to remote: {:?}", msg);
        match msg {
            ServerToRemote::Heartbeat => {
                // info!("Server heartbeat");
                if self
                    .inner
                    .send(&CommandResponse::new_heartbeat())
                    .await
                    .is_err()
                {
                    error!("Server send heartbeat failed");
                }
            }
            ServerToRemote::ClosePort(id) => {
                info!("Server Close port id: {}", id);
                if let Some(tx) = server_port_map.get_mut(&id) {
                    if tx.send(RemoteMsg::ClosePort(id)).await.is_err() {
                        info!("Server send close failed");
                    }
                }
                server_port_map.remove(&id);
            }
            ServerToRemote::TcpConnect(id, target_addr) => {
                let (tx, rx) = channel(1000);
                server_port_map.insert(id, tx);
                let sender = self.subsenders.get_one_sender();
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
                    tcp_tunnel_port_task(target_addr, reader_remote, writer_remote).await;
                });
            }
            ServerToRemote::UdpAssociate(id) => {
                info!("Server udp connect id: {}", id);
                let (tx, rx) = channel(1000);
                server_port_map.insert(id, tx);
                let sender = self.subsenders.get_one_sender();
                let read_port = TunnelReader {
                    id,
                    tx: sender.clone(),
                    rx: Some(rx),
                };

                let write_port = TunnelWriter {
                    id,
                    tx: sender.clone(),
                };

                tokio::spawn(async move {
                    udp_tunnel_port_task(read_port, write_port).await;
                });
            }
            ServerToRemote::Data(id, data) => {
                info!("Server get data: {:?}", data.len());
                if let Some(tx) = server_port_map.get_mut(&id) {
                    if tx.send(RemoteMsg::Data(data)).await.is_err() {
                        info!("Server send data failed");
                        server_port_map.remove(&id);
                    }
                }
            }
            ServerToRemote::UdpData(id, target_addr, data) => {
                info!("Server get udp data: {:?}", data.len());
                if let Some(tx) = server_port_map.get_mut(&id) {
                    if tx
                        .send(RemoteMsg::UdpData(target_addr, data))
                        .await
                        .is_err()
                    {
                        info!("Server send udp data failed");
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
    ) {
        match msg {
            RemoteToServer::TcpConnectSuccess(id, bind_addr) => {
                debug!("Remote connect success id: {}", id);
                if self
                    .inner
                    .send(&CommandResponse::new_tcp_connect_success(id, bind_addr))
                    .await
                    .is_err()
                {
                    error!("Server send tcp connect success failed");
                    server_port_map.remove(&id);
                }
            }
            RemoteToServer::TcpConnectFailed(id) => {
                info!("Remote connect failed: {}", id);
                if self
                    .inner
                    .send(&CommandResponse::new_tcp_connect_failed(id))
                    .await
                    .is_err()
                {
                    error!("Server send tcp connect failed failed");
                    server_port_map.remove(&id);
                }
            }
            RemoteToServer::UdpAssociateSuccess(id) => {
                debug!("Remote udp connect success: {}", id);
                if self
                    .inner
                    .send(&CommandResponse::new_udp_associate_success(id))
                    .await
                    .is_err()
                {
                    error!("Server send udp connect success failed");
                    server_port_map.remove(&id);
                }
            }
            RemoteToServer::UdpAssociateFailed(id) => {
                info!("Remote udp connect failed: {}", id);
                if self
                    .inner
                    .send(&CommandResponse::new_udp_associate_failed(id))
                    .await
                    .is_err()
                {
                    error!("Server send udp connect failed failed");
                    server_port_map.remove(&id);
                }
            }
            RemoteToServer::Data(id, data) => {
                debug!("Remote get id: {}, data len: {:?}", id, data.len());
                let msg = CommandResponse::new_data(id, *data);
                if self.inner.send(&msg).await.is_err() {
                    error!("Server send data failed");
                    server_port_map.remove(&id);
                }
            }
            RemoteToServer::UdpData(id, target_addr, data) => {
                debug!("Remote get udp data: {}, {:?}", id, data.len());
                let msg = CommandResponse::new_udp_data(id, target_addr, *data);
                if self.inner.send(&msg).await.is_err() {
                    error!("Server send udp data failed");
                    server_port_map.remove(&id);
                }
            }
            RemoteToServer::ClosePort(id) => {
                info!("Remote close port: {}", id);
                if self
                    .inner
                    .send(&CommandResponse::new_close_port(id))
                    .await
                    .is_err()
                {
                    error!("Server send close port failed");
                    server_port_map.remove(&id);
                }
            }
        }
    }
}
