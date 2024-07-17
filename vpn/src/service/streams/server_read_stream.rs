use anyhow::Result;
use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use tokio::io::AsyncRead;
use tracing::{debug, error, info, warn};

use crate::{
    pb::{command_request::Command, CommandRequest},
    util::TargetAddr,
    ProstReadStream, ServerMsg, ServerToRemote, VpnError,
};

pub struct VpnServerProstReadStream<S> {
    pub inner: ProstReadStream<S, CommandRequest>,
}

impl<S> VpnServerProstReadStream<S>
where
    S: AsyncRead + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: ProstReadStream::new(stream),
        }
    }

    pub async fn next(&mut self) -> Option<Result<CommandRequest, VpnError>> {
        self.inner.next().await
    }

    pub async fn process(&mut self, mut sender: Sender<ServerMsg>) -> Result<(), VpnError> {
        while let Some(Ok(req)) = self.next().await {
            match req.command {
                Some(Command::Heartbeat(_)) => {
                    info!("server read stream get heartbeat");
                    if sender.send(ServerToRemote::Heartbeat.into()).await.is_err() {
                        error!("send heartbeat to remote failed");
                    }
                }
                Some(Command::TcpConnect(tcp_connect)) => match tcp_connect.destination {
                    None => {
                        warn!("tcp connect destination is none");
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
