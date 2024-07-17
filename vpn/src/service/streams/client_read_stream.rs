use std::sync::Arc;

use anyhow::Result;

use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use tokio::io::AsyncRead;
use tracing::{info, warn};

use crate::{
    pb::{
        command_response::Response::{self},
        CommandResponse,
    },
    ClientMsg, ClientPortMap, ClientToSocks5Msg, ProstReadStream, ServiceError, VpnError,
};

pub struct VpnClientProstReadStream<S> {
    pub inner: ProstReadStream<S, CommandResponse>,
}

impl<S> VpnClientProstReadStream<S>
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

    pub async fn process(
        &mut self,
        mut sender: Sender<ClientMsg>,
        _channel_map: Arc<ClientPortMap>,
    ) -> Result<(), VpnError> {
        while let Some(Ok(res)) = self.next().await {
            match res.response {
                Some(Response::Data(data)) => {
                    info!(
                        "Client read stream get data, id: {}, {:?}",
                        data.id, data.data
                    );
                    if sender
                        .send(ClientToSocks5Msg::Data(data.id, data.data).into())
                        .await
                        .is_err()
                    {
                        warn!("Send data to socks5 failed");
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
