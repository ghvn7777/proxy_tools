mod socks5_stream;

use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    pb::{VpnCommandRequest, VpnCommandResponse},
    ProstStream, Socks5Config, VpnError, YamuxCtrl,
};
pub use socks5_stream::Socks5Stream;

pub struct Socks5ServerStream<S> {
    inner: Socks5Stream<S>,
    config: Arc<Socks5Config>,
}

pub struct ProstClientStream<S> {
    inner: ProstStream<S, VpnCommandResponse, VpnCommandRequest>,
}

impl<S> Socks5ServerStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(stream: S, config: Arc<Socks5Config>) -> Self {
        Self {
            inner: Socks5Stream::new(stream),
            config,
        }
    }

    pub async fn process(self) -> Result<(), VpnError> {
        trace!("Socks5ServerStream: processing...");
        let uuid = Uuid::new_v4().to_string();

        let mut stream = self.inner;
        stream.handshake(&self.config.auth_type).await?;
        let (_cmd, target_addr) = stream.request(self.config.dns_resolve).await?;
        debug!("request good");

        // todo: loop get stream send to server
        let addr = "127.0.0.1:9527";
        let client_stream = TcpStream::connect(addr).await?;
        let mut ctrl = YamuxCtrl::new_client(client_stream, None);
        let client_stream = ctrl.open_stream().await?;

        let mut client = ProstClientStream::new(client_stream);
        let cmd = VpnCommandRequest::new_connect(uuid.clone(), target_addr);
        info!("Send a new cmd: {:?}", cmd);
        let res = client.execute_unary(&cmd).await?;

        info!("Got a new res: {:?}", res);
        // todo: drop this stream and close connection

        Ok(())
    }
}

impl<S> ProstClientStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: ProstStream::new(stream),
        }
    }

    pub async fn execute_unary(
        &mut self,
        cmd: &VpnCommandRequest,
    ) -> Result<VpnCommandResponse, VpnError> {
        trace!("ProstClientStream: execute_unary");
        let stream = &mut self.inner;
        stream.send(cmd).await?;

        match stream.next().await {
            Some(v) => v,
            None => Err(VpnError::InternalError("Didn't get any response".into())),
        }
    }

    // pub async fn execute_streaming(self, cmd: &CommandRequest) -> Result<StreamResult, KvError> {
    //     let mut stream = self.inner;

    //     stream.send(cmd).await?;
    //     stream.close().await?;

    //     StreamResult::new(stream).await
    // }
}
