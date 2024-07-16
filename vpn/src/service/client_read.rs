use anyhow::Result;

use futures::{channel::mpsc::Sender, StreamExt};
use tokio::io::AsyncRead;
use tracing::warn;

use crate::{pb::CommandResponse, ProstReadStream, ServiceError, TunnelMsg, VpnError};

pub struct VpnProstReadStream<S> {
    pub inner: ProstReadStream<S, CommandResponse>,
}

impl<S> VpnProstReadStream<S>
where
    S: AsyncRead + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: ProstReadStream::new(stream),
        }
    }

    pub async fn next(&mut self) -> Option<Result<CommandResponse, VpnError>> {
        self.inner.next().await
    }

    pub async fn process(&mut self, _sender: Sender<TunnelMsg>) -> Result<(), VpnError> {
        while let Some(Ok(res)) = self.next().await {
            match res.response {
                Some(_) => {
                    todo!()
                }
                _ => {
                    warn!("Got an unknown response: {:?}", res);
                    return Err(ServiceError::UnknownCommand("Unknown response".to_string()).into());
                }
            }
        }
        Ok(())
    }
}
