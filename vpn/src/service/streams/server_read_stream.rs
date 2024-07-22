use std::sync::Arc;

use anyhow::Result;
use futures::{channel::mpsc::Sender, SinkExt};
use prost::Message;
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};
use tracing::{debug, error, info, warn};

use crate::{
    pb::{command_request::Command, CommandRequest},
    util::TargetAddr,
    DataCrypt, ServerMsg, ServerToRemote, ServiceError, VpnError,
};

pub struct TcpServerProstReadStream {
    pub inner: OwnedReadHalf,
    pub crypt: Arc<Box<dyn DataCrypt>>,
}

impl TcpServerProstReadStream {
    pub fn new(stream: OwnedReadHalf, crypt: Arc<Box<dyn DataCrypt>>) -> Self {
        Self {
            inner: stream,
            crypt,
        }
    }

    pub async fn next(&mut self) -> Result<CommandRequest, VpnError> {
        let len = self.inner.read_i32().await? as usize;
        let mut buf = vec![0; len];
        self.inner.read_exact(&mut buf).await?;

        // info!("before decrypt buf: {:?}", buf);
        let Ok(buf) = self.crypt.as_ref().decrypt(&buf) else {
            error!("decrypt error (maybe the crypt key is wrong)");
            return Err(ServiceError::DecryptError.into());
        };
        // info!("after decrypt buf: {:?}", buf);

        let msg: CommandRequest = match Message::decode(&buf[..]) {
            Ok(msg) => msg,
            Err(e) => {
                error!("decode error (maybe the crypt key is wrong): {:?}", e);
                return Err(e.into());
            }
        };
        Ok(msg)
    }

    pub async fn process(&mut self, mut sender: Sender<ServerMsg>) -> Result<(), VpnError> {
        while let Ok(req) = self.next().await {
            match req.command {
                Some(Command::Heartbeat(_)) => {
                    // info!("server read stream get heartbeat");
                    if sender.send(ServerToRemote::Heartbeat.into()).await.is_err() {
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
                        if sender
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
                    if sender
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
                        if sender
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
                    if sender
                        .send(ServerToRemote::ClosePort(id).into())
                        .await
                        .is_err()
                    {
                        error!("send close port to remote failed");
                    }
                }
                Some(Command::Data(data)) => {
                    debug!("stream reader data len: {:?}", data.data.len());
                    if sender
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
