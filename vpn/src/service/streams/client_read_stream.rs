use anyhow::Result;

use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use tokio::io::AsyncRead;
use tracing::{info, warn};

use crate::{
    pb::{
        command_response::Response::{self},
        CommandResponse,
    },
    ClientMsg, ClientToSocks5Msg, ProstReadStream, ServiceError, VpnError,
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

    pub async fn process(&mut self, mut sender: Sender<ClientMsg>) -> Result<(), VpnError> {
        while let Some(Ok(res)) = self.next().await {
            match res.response {
                Some(Response::Heartbeat(_)) => {
                    info!("Client read stream get heartbeat");
                    if sender
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
                        data.id, data.data
                    );
                    if sender
                        .send(ClientToSocks5Msg::Data(data.id, Box::new(data.data)).into())
                        .await
                        .is_err()
                    {
                        warn!("Send data to socks5 failed");
                    }
                }
                Some(Response::ClosePort(port)) => {
                    info!("Client read stream get close port, id: {}", port);
                    if sender
                        .send(ClientToSocks5Msg::ClosePort(port).into())
                        .await
                        .is_err()
                    {
                        warn!("Send close port to socks5 failed");
                    }
                }
                Some(Response::TcpConnectSuccess(id)) => {
                    info!("Client read stream get tcp connect success, id: {}", id);
                    if sender
                        .send(ClientToSocks5Msg::TcpConnectSuccess(id).into())
                        .await
                        .is_err()
                    {
                        warn!("Send tcp connect success to socks5 failed");
                    }
                }
                Some(Response::TcpConnectFailed(id)) => {
                    info!("Client read stream get tcp connect failed, id: {}", id);
                    if sender
                        .send(ClientToSocks5Msg::TcpConnectFailed(id).into())
                        .await
                        .is_err()
                    {
                        warn!("Send tcp connect failed to socks5 failed");
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
