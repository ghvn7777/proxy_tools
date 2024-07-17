use anyhow::Result;
use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use tokio::io::AsyncRead;
use tracing::{info, warn};

use crate::{
    pb::{command_request::Command, CommandRequest},
    util::TargetAddr,
    ProstReadStream, ServerMsg, VpnError,
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
                Some(Command::TcpConnect(tcp_connect)) => match tcp_connect.destination {
                    None => {
                        warn!("tcp connect destination is none");
                        continue;
                    }
                    Some(target_addr) => {
                        let target_addr: TargetAddr = target_addr.try_into().unwrap();
                        let id = tcp_connect.id;
                        info!("tcp connect: {}, {:?}", id, target_addr);
                        let _ = sender.send(ServerMsg::TcpConnect(id, target_addr)).await;
                    }
                },
                Some(Command::ClosePort(id)) => {
                    info!("close port id: {}", id);
                    let _ = sender.send(ServerMsg::ClosePort(id)).await;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
