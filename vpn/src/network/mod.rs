mod frame;
mod multiplex;
mod stream;
mod tls;

use std::{borrow::BorrowMut, sync::Arc, time::Duration};

use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    net::TcpStream,
    time::timeout,
};
use tracing::{info, trace};

use crate::{
    pb::{VpnCommandRequest, VpnCommandResponse},
    VpnError,
};
pub use frame::{read_frame, FrameCoder};
pub use multiplex::YamuxCtrl;
pub use stream::ProstStream;
pub use tls::{TlsClientConnector, TlsServerAcceptor};

pub type StreamMap = Arc<DashMap<String, TcpStream>>;

pub struct ProstServerStream<S> {
    inner: ProstStream<S, VpnCommandRequest, VpnCommandResponse>,
    streams: StreamMap,
}

impl<S> ProstServerStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(stream: S, streams: StreamMap) -> Self {
        Self {
            inner: ProstStream::new(stream),
            streams,
        }
    }

    pub async fn process(mut self) -> Result<(), VpnError> {
        trace!("ProstServerStream: processing...");
        let stream = &mut self.inner;
        let mut connect_id = "".to_string();

        let mut buf = vec![0u8; 1024 * 1000]; // 1000 M
        loop {
            let res = stream.next();
            let res = match timeout(Duration::from_millis(20), res).await {
                Ok(res) => res,
                Err(_) => None,
            };
            if let Some(Ok(cmd)) = res {
                connect_id.clone_from(&cmd.connect_id);
                let res = cmd.execute(&self.streams, &cmd.connect_id).await?;
                info!("response: {:?}", &res);
                stream.send(&res).await?;
            }

            if let Some(mut des_stream) = self.streams.get_mut(&connect_id) {
                let res = (*des_stream).borrow_mut().read(&mut buf);
                if let Ok(res) = timeout(Duration::from_millis(20), res).await {
                    if res.is_ok() {
                        let n = res.unwrap();
                        if n == 0 {
                            continue;
                        }
                        let ret = VpnCommandResponse::new_data(buf[..n].to_vec());
                        stream.send(&ret).await?;
                    }
                };
            }
        }

        #[allow(unreachable_code)]
        Ok(())
    }
}
