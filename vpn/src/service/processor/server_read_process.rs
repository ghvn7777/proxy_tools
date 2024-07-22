use std::sync::Arc;

use anyhow::Result;
use futures::{channel::mpsc::Sender, SinkExt};
use tokio::io::AsyncReadExt;
use tonic::async_trait;
use tracing::{debug, error, info, warn};

use crate::{
    pb::{command_request::Command, CommandRequest},
    util::TargetAddr,
    DataCrypt, ProstReadStream, ReadStream, ServerMsg, ServerToRemote, VpnError,
};

use super::Processor;

pub struct ServerReadProcessor<T> {
    pub inner: ProstReadStream<CommandRequest, T>,
    sender: Sender<ServerMsg>,
}

impl<T> ServerReadProcessor<T> {
    pub fn new(stream: T, crypt: Arc<Box<dyn DataCrypt>>, sender: Sender<ServerMsg>) -> Self {
        Self {
            inner: ProstReadStream::new(stream, crypt),
            sender,
        }
    }
}

#[async_trait]
impl<T> Processor for ServerReadProcessor<T>
where
    T: AsyncReadExt + Unpin + Send + Sync + 'static,
{
    async fn process(&mut self) -> Result<(), VpnError> {
        while let Ok(req) = self.inner.next().await {
            match req.command {
                Some(Command::Heartbeat(_)) => {
                    // info!("server read stream get heartbeat");
                    if self
                        .sender
                        .send(ServerToRemote::Heartbeat.into())
                        .await
                        .is_err()
                    {
                        error!("send heartbeat to remote failed");
                    }
                    continue;
                }
                Some(Command::TcpConnect(tcp_connect)) => match tcp_connect.destination {
                    None => {
                        error!("tcp connect destination is none");
                        continue;
                    }
                    Some(target_addr) => {
                        let target_addr: TargetAddr = target_addr.try_into().unwrap();
                        let id = tcp_connect.id;
                        info!("tcp connect: {}, {:?}", id, target_addr);
                        if self
                            .sender
                            .send(ServerToRemote::TcpConnect(id, target_addr).into())
                            .await
                            .is_err()
                        {
                            error!("send tcp connect to remote failed");
                        }
                    }
                },
                Some(Command::UdpAssociate(id)) => {
                    debug!("server read stream udp connect id: {}", id);
                    if self
                        .sender
                        .send(ServerToRemote::UdpAssociate(id).into())
                        .await
                        .is_err()
                    {
                        error!("send udp connect to remote failed");
                    }
                }
                Some(Command::UdpData(udp_data)) => match udp_data.destination {
                    None => {
                        error!("udp data destination is none");
                        continue;
                    }
                    Some(target_addr) => {
                        let target_addr: TargetAddr = target_addr.try_into().unwrap();
                        let id = udp_data.id;
                        info!(
                            "udp data: {}, {:?}, {:?}",
                            id,
                            target_addr,
                            udp_data.data.len()
                        );
                        if self
                            .sender
                            .send(
                                ServerToRemote::UdpData(id, target_addr, Box::new(udp_data.data))
                                    .into(),
                            )
                            .await
                            .is_err()
                        {
                            error!("send udp data to remote failed");
                        }
                    }
                },
                Some(Command::ClosePort(id)) => {
                    debug!("stream reader close port id: {}", id);
                    if self
                        .sender
                        .send(ServerToRemote::ClosePort(id).into())
                        .await
                        .is_err()
                    {
                        error!("send close port to remote failed");
                    }
                }
                Some(Command::Data(data)) => {
                    debug!("stream reader data len: {:?}", data.data.len());
                    if self
                        .sender
                        .send(ServerToRemote::Data(data.id, Box::new(data.data)).into())
                        .await
                        .is_err()
                    {
                        error!("send data to remote failed");
                    }
                }
                None => {
                    warn!("stream reader get none");
                }
            }
        }
        Ok(())
    }
}
