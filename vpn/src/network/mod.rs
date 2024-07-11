mod frame;
mod multiplex;
mod stream;

use std::sync::Arc;

use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tracing::{info, trace};

use crate::{
    pb::{VpnCommandRequest, VpnCommandResponse},
    VpnError,
};
pub use frame::{read_frame, FrameCoder};
pub use multiplex::YamuxCtrl;
pub use stream::ProstStream;

pub type StreamMap = Arc<DashMap<String, TcpStream>>;

pub struct ProstServerStream<S> {
    inner: ProstStream<S, VpnCommandRequest, VpnCommandResponse>,
    _streams: StreamMap,
}

impl<S> ProstServerStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(stream: S, streams: StreamMap) -> Self {
        Self {
            inner: ProstStream::new(stream),
            _streams: streams,
        }
    }

    pub async fn process(mut self) -> Result<(), VpnError> {
        trace!("ProstServerStream: processing...");
        let stream = &mut self.inner;
        while let Some(Ok(cmd)) = stream.next().await {
            info!("Got a new command: {:?}", cmd);
            let res = VpnCommandResponse::new_error("hello".to_string());
            info!("response: {:?}", &res);
            stream.send(&res).await?;

            // let mut res = cmd.execute().await?;
            // tokio::spawn(async move { while let Some(ret) = res.next().await {} });
        }
        // info!("Client {:?} disconnected", self.addr);
        Ok(())
    }
}
