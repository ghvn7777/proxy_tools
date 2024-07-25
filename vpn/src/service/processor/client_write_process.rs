use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use futures::{SinkExt, Stream, StreamExt};
use tokio::io::AsyncWriteExt;
use tonic::async_trait;
use tracing::{debug, error, info, trace, warn};

use crate::{
    pb::CommandRequest, ClientMsg, ClientPortMap, ClientToSocks5Msg, DataCrypt, ProstWriteStream,
    ServiceError, Socks5ToClientMsg, SocksMsg, VpnError, WriteStream, ALIVE_TIMEOUT_TIME_MS,
};

use super::Processor;

pub struct ClientWriteProcessor<'a, T, V> {
    inner: ProstWriteStream<CommandRequest, T>,
    msg_stream: &'a mut V,
}

impl<'a, T, V> ClientWriteProcessor<'a, T, V> {
    pub fn new(stream: T, crypt: Arc<Box<dyn DataCrypt>>, msg_stream: &'a mut V) -> Self {
        Self {
            inner: ProstWriteStream::new(stream, crypt),
            msg_stream,
        }
    }
}

#[async_trait]
impl<'a, T, V> Processor for ClientWriteProcessor<'a, T, V>
where
    T: AsyncWriteExt + Unpin + Send + Sync + 'static,
    V: Stream<Item = ClientMsg> + Unpin + Send + Sync + 'static,
{
    async fn process(&mut self) -> Result<(), VpnError> {
        let mut port_map = HashMap::new();
        let mut alive_time = Instant::now();
        loop {
            match self.msg_stream.next().await {
                Some(msg) => match msg {
                    ClientMsg::Heartbeat => {
                        // info!("Client Msg receive heartbeat");
                        if Instant::now() - alive_time
                            > Duration::from_millis(ALIVE_TIMEOUT_TIME_MS)
                        {
                            error!("Client heartbeat timeout");
                            break;
                        }
                        let msg = CommandRequest::new_heartbeat();
                        self.inner.send(&msg).await?;
                    }
                    ClientMsg::Socks5ToClient(msg) => {
                        alive_time = Instant::now();
                        self.process_socks5_to_client(msg, &mut port_map).await?
                    }
                    ClientMsg::ClientToSocks5(msg) => {
                        alive_time = Instant::now();
                        self.process_client_to_socks5(msg, &mut port_map).await?
                    }
                },
                None => {
                    error!("Tunnel get none message, stop processing...");
                    break;
                }
            }
        }

        Ok(())
    }
}

impl<'a, T, V> ClientWriteProcessor<'a, T, V>
where
    T: AsyncWriteExt + Unpin + Send + Sync + 'static,
    V: Stream<Item = ClientMsg> + Unpin,
{
    pub async fn process_socks5_to_client(
        &mut self,
        msg: Socks5ToClientMsg,
        port_map: &mut ClientPortMap,
    ) -> Result<(), VpnError> {
        trace!("process_socks5_to_client msg: {:?}", msg);
        match msg {
            Socks5ToClientMsg::InitChannel(id, tx) => {
                if port_map.insert(id, tx).is_some() {
                    error!("Init channel failed, channel id already exists");
                    return Err(ServiceError::ChannelIdExists(id).into());
                }
                debug!("process_socks5_to_client: Init channel: {:?}", port_map);
            }
            Socks5ToClientMsg::ClosePort(id) => {
                debug!("Close port {}: {:?}", id, port_map);
                // 由 socks5 发出来的，说明 socks5 关闭了这个端口，所以这里直接把这个端口从 map 中移除就行
                if port_map.remove(&id).is_none() {
                    warn!("process_socks5_to_client: Port id not found: {}", id);
                }
                let msg = CommandRequest::new_close_port(id);
                self.inner.send(&msg).await?;
            }
            Socks5ToClientMsg::TcpConnect(id, addr) => {
                debug!("process_socks5_to_client: TcpConnect: {} -- {:?}", id, addr);
                let msg = CommandRequest::new_tcp_connect(id, addr);
                self.inner.send(&msg).await?;
            }
            Socks5ToClientMsg::UdpAssociate(id) => {
                debug!("process_socks5_to_client: UdpAssociate: {}", id);
                let msg = CommandRequest::new_associate_connect(id);
                self.inner.send(&msg).await?;
            }
            Socks5ToClientMsg::Data(id, data) => {
                info!(
                    "process_socks5_to_client data: id = {}, data_len = {:?}",
                    id,
                    data.len()
                );
                let msg = CommandRequest::new_data(id, *data);
                self.inner.send(&msg).await?;
            }
            Socks5ToClientMsg::UdpData(id, target_addr, data) => {
                // info!(
                //     "process_socks5_to_client udp data: {}, {:?}, {:?}",
                //     id, target_addr, data
                // );
                let msg = CommandRequest::new_udp_data(id, target_addr, *data);
                self.inner.send(&msg).await?;
            }
        }
        Ok(())
    }

    pub async fn process_client_to_socks5(
        &mut self,
        msg: ClientToSocks5Msg,
        port_map: &mut ClientPortMap,
    ) -> Result<(), VpnError> {
        // trace!("process_client_to_socks5: {:?}", msg);
        match msg {
            ClientToSocks5Msg::Heartbeat => {
                // 啥也不用干，在上面更新了时间
                // info!("process client: Client WriteStream receive heartbeat");
            }
            ClientToSocks5Msg::Data(id, data) => {
                // debug!("process_client_to_socks5 data: {}, {:?}", id, data);
                if let Some(tx) = port_map.get_mut(&id) {
                    if tx.send(SocksMsg::Data(data)).await.is_err() {
                        error!("process_client_to_socks5: Send data failed");
                        port_map.remove(&id);
                    }
                } else {
                    warn!("process_client_to_socks5: Port id not found: {}", id);
                }
            }
            ClientToSocks5Msg::UdpData(id, target_addr, data) => {
                debug!(
                    "process_client_to_socks5 udp data: {}, {:?}",
                    id,
                    data.len()
                );
                if let Some(tx) = port_map.get_mut(&id) {
                    if tx.send(SocksMsg::UdpData(target_addr, data)).await.is_err() {
                        error!("process_client_to_socks5: Send udp data failed");
                        port_map.remove(&id);
                    }
                } else {
                    warn!("process_client_to_socks5: Port id not found: {}", id);
                }
            }
            ClientToSocks5Msg::ClosePort(id) => {
                debug!("process_client_to_socks5: Close port: {}", id);
                // 由服务端发过来的，要通知 socks5 关闭这个端口
                if let Some(tx) = port_map.get_mut(&id) {
                    if tx.send(SocksMsg::ClosePort(id)).await.is_err() {
                        error!("process_client_to_socks5: Send data failed");
                    }
                } else {
                    warn!("process_client_to_socks5: close port id not found: {}", id);
                }
                port_map.remove(&id);
            }
            ClientToSocks5Msg::TcpConnectSuccess(id, bind_addr) => {
                debug!("process_client_to_socks5: TcpConnectSuccess: {}", id);
                if let Some(tx) = port_map.get_mut(&id) {
                    if tx
                        .send(SocksMsg::TcpConnectSuccess(id, bind_addr))
                        .await
                        .is_err()
                    {
                        error!("process_client_to_socks5: Send TcpConnectSuccess failed");
                        port_map.remove(&id);
                    }
                } else {
                    warn!("process_client_to_socks5: Port id not found: {}", id);
                }
            }
            ClientToSocks5Msg::TcpConnectFailed(id) => {
                info!("process_client_to_socks5: TcpConnectFailed: {}", id);
                if let Some(tx) = port_map.get_mut(&id) {
                    if tx.send(SocksMsg::TcpConnectFailed(id)).await.is_err() {
                        error!("process_client_to_socks5: Send TcpConnectFailed failed");
                        port_map.remove(&id);
                    }
                } else {
                    warn!("process_client_to_socks5: Port id not found: {}", id);
                }
            }
            ClientToSocks5Msg::UdpAssociateSuccess(id) => {
                debug!("process_client_to_socks5: UdpAssociateSuccess: {}", id);
                if let Some(tx) = port_map.get_mut(&id) {
                    if tx.send(SocksMsg::UdpAssociateSuccess(id)).await.is_err() {
                        error!("process_client_to_socks5: Send UdpAssociateSuccess msg failed");
                        port_map.remove(&id);
                    }
                } else {
                    warn!("process_client_to_socks5: Port id not found: {}", id);
                }
            }
            ClientToSocks5Msg::UdpAssociateFailed(id) => {
                info!("process_client_to_socks5: UdpAssociateFailed: {}", id);
                if let Some(tx) = port_map.get_mut(&id) {
                    if tx.send(SocksMsg::UdpAssociateFailed(id)).await.is_err() {
                        error!("process_client_to_socks5: Send UdpAssociateFailed msg failed");
                        port_map.remove(&id);
                    }
                } else {
                    warn!("process_client_to_socks5: Port id not found: {}", id);
                }
            }
        }
        Ok(())
    }
}
