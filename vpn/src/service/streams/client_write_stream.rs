use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use futures::{SinkExt, Stream, StreamExt};
use tokio::io::AsyncWrite;
use tracing::{debug, error, info, trace, warn};

use crate::{
    pb::CommandRequest, ClientMsg, ClientPortMap, ClientToSocks5Msg, ProstWriteStream,
    ServiceError, Socks5ToClientMsg, SocksMsg, VpnError, ALIVE_TIMEOUT_TIME_MS,
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

    pub async fn process<T>(&mut self, mut msg_stream: T) -> Result<(), VpnError>
    where
        T: Stream<Item = ClientMsg> + Unpin,
    {
        let mut port_map = HashMap::new();
        let mut alive_time = Instant::now();
        loop {
            match msg_stream.next().await {
                Some(msg) => match msg {
                    ClientMsg::Heartbeat => {
                        info!("Client Msg receive heartbeat");
                        if Instant::now() - alive_time
                            > Duration::from_millis(ALIVE_TIMEOUT_TIME_MS)
                        {
                            error!("Client heartbeat timeout");
                            break;
                        }
                        let msg = CommandRequest::new_heartbeat();
                        self.send(&msg).await?;
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

    pub async fn process_socks5_to_client(
        &mut self,
        msg: Socks5ToClientMsg,
        port_map: &mut ClientPortMap,
    ) -> Result<(), VpnError> {
        trace!("process_socks5_to_client: {:?}", msg);
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
                self.send(&msg).await?;
            }
            Socks5ToClientMsg::TcpConnect(id, addr) => {
                debug!("process_socks5_to_client: TcpConnect: {} -- {:?}", id, addr);
                let msg = CommandRequest::new_tcp_connect(id, addr);
                self.send(&msg).await?;
            }
            Socks5ToClientMsg::Data(id, data) => {
                info!("process_socks5_to_client data: {}, {:?}", id, data);
                let msg = CommandRequest::new_data(id, *data);
                self.send(&msg).await?;
            }
        }
        Ok(())
    }

    pub async fn process_client_to_socks5(
        &mut self,
        msg: ClientToSocks5Msg,
        port_map: &mut ClientPortMap,
    ) -> Result<(), VpnError> {
        trace!("process_client_to_socks5: {:?}", msg);
        match msg {
            ClientToSocks5Msg::Heartbeat => {
                // 啥也不用干，在上面更新了时间
                info!("process client: Client WriteStream receive heartbeat");
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
        }
        Ok(())
    }
}
