use std::time::Instant;

use futures::{SinkExt, Stream, StreamExt};
use tokio::io::AsyncWrite;
use tracing::error;

use crate::{
    pb::CommandRequest, tunnel::PortHub, ProstWriteStream, TunnelMsg, VpnError,
    ALIVE_TIMEOUT_TIME_MS,
};

// TODO: read 和 write stream 里的 vpn 类型给反了
pub struct VpnProstWriteStream<S> {
    inner: ProstWriteStream<S, CommandRequest>,
}

impl<S> VpnProstWriteStream<S>
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

    pub async fn process<T>(
        &mut self,
        mut msg_stream: T,
        _port_hub: PortHub,
    ) -> Result<(), VpnError>
    where
        T: Stream<Item = TunnelMsg> + Unpin,
    {
        let alive_time = Instant::now();
        loop {
            match msg_stream.next().await {
                Some(msg) => match msg {
                    TunnelMsg::Heartbeat => {
                        let now = Instant::now();
                        let duration = now.duration_since(alive_time);
                        if duration.as_millis() > ALIVE_TIMEOUT_TIME_MS {
                            error!(
                                "Heartbeat timeout, stop processing: {:?} - {:?}",
                                now, alive_time
                            );
                            break;
                        }
                    }
                    _ => break,
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
