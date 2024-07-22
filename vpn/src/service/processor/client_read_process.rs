use std::sync::Arc;

use anyhow::Result;

use futures::{channel::mpsc::Sender, SinkExt};
use tokio::io::AsyncReadExt;
use tonic::async_trait;
use tracing::{error, info, warn};

use crate::{
    pb::{
        command_response::Response::{self},
        CommandResponse, TcpConnectSuccess,
    },
    util::TargetAddr,
    ClientMsg, ClientToSocks5Msg, DataCrypt, ProstReadStream, ReadStream, ServiceError, VpnError,
};

use super::Processor;

pub struct ClientReadProcessor<T> {
    pub inner: ProstReadStream<CommandResponse, T>,
    sender: Sender<ClientMsg>,
}

impl<T> ClientReadProcessor<T> {
    pub fn new(stream: T, crypt: Arc<Box<dyn DataCrypt>>, sender: Sender<ClientMsg>) -> Self {
        Self {
            inner: ProstReadStream::new(stream, crypt),
            sender,
        }
    }
}

#[async_trait]
impl<T> Processor for ClientReadProcessor<T>
where
    T: AsyncReadExt + Unpin + Send + Sync + 'static,
{
    async fn process(&mut self) -> Result<(), VpnError> {
        while let Ok(res) = self.inner.next().await {
            match res.response {
                Some(Response::Heartbeat(_)) => {
                    // info!("Client read stream get heartbeat");
                    if self
                        .sender
                        .send(ClientToSocks5Msg::Heartbeat.into())
                        .await
                        .is_err()
                    {
                        warn!("Send heartbeat to socks5 failed");
                    }
                }
                Some(Response::Data(data)) => {
                    info!(
                        "Client read stream get data, id: {}, {:?}",
                        data.id,
                        data.data.len()
                    );
                    if self
                        .sender
                        .send(ClientToSocks5Msg::Data(data.id, Box::new(data.data)).into())
                        .await
                        .is_err()
                    {
                        warn!("Send data to socks5 failed");
                    }
                }
                Some(Response::UdpData(udp_data)) => {
                    info!(
                        "Client read stream get udp data, id: {}, {:?}, {:?}",
                        udp_data.id,
                        udp_data.destination,
                        udp_data.data.len()
                    );
                    let destination = match udp_data.destination {
                        Some(udp_destination) => udp_destination.try_into().unwrap(),
                        None => {
                            error!("Udp data destination is none");
                            if self
                                .sender
                                .send(ClientToSocks5Msg::ClosePort(udp_data.id).into())
                                .await
                                .is_err()
                            {
                                warn!("Send close port to socks5 failed");
                            }
                            continue;
                        }
                    };

                    if self
                        .sender
                        .send(
                            ClientToSocks5Msg::UdpData(
                                udp_data.id,
                                destination,
                                Box::new(udp_data.data),
                            )
                            .into(),
                        )
                        .await
                        .is_err()
                    {
                        warn!("Send udp data to socks5 failed");
                    }
                }
                Some(Response::ClosePort(port)) => {
                    info!("Client read stream get close port, id: {}", port);
                    if self
                        .sender
                        .send(ClientToSocks5Msg::ClosePort(port).into())
                        .await
                        .is_err()
                    {
                        warn!("Send close port to socks5 failed");
                    }
                }
                Some(Response::TcpConnectSuccess(TcpConnectSuccess { id, bind_addr })) => {
                    info!(
                        "Client read stream get tcp connect success: {}, {:?}",
                        id, bind_addr
                    );
                    match bind_addr {
                        Some(bind_addr) => {
                            let bind_addr: TargetAddr = bind_addr.try_into().unwrap();
                            if self
                                .sender
                                .send(ClientToSocks5Msg::TcpConnectSuccess(id, bind_addr).into())
                                .await
                                .is_err()
                            {
                                warn!("Send tcp connect success to socks5 failed");
                            }
                        }
                        None => {
                            error!("Tcp connect success bind addr is none");
                        }
                    }
                }
                Some(Response::TcpConnectFailed(id)) => {
                    info!("Client read stream get tcp connect failed, id: {}", id);
                    if self
                        .sender
                        .send(ClientToSocks5Msg::TcpConnectFailed(id).into())
                        .await
                        .is_err()
                    {
                        warn!("Send tcp connect failed to socks5 failed");
                    }
                }
                Some(Response::UdpAssociateSuccess(id)) => {
                    info!("Client read stream get udp associate success, id: {}", id);
                    if self
                        .sender
                        .send(ClientToSocks5Msg::UdpAssociateSuccess(id).into())
                        .await
                        .is_err()
                    {
                        warn!("Send udp associate success to socks5 failed");
                    }
                }
                Some(Response::UdpAssociateFailed(id)) => {
                    info!("Client read stream get udp associate failed, id: {}", id);
                    if self
                        .sender
                        .send(ClientToSocks5Msg::UdpAssociateFailed(id).into())
                        .await
                        .is_err()
                    {
                        warn!("Send udp associate failed to socks5 failed");
                    }
                }
                None => {
                    warn!("Got an unknown response: {:?}", res);
                    return Err(ServiceError::UnknownCommand("Unknown response".to_string()).into());
                }
            }
        }
        Ok(())
    }
}
