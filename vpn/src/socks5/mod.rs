mod socks5_stream;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use futures::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    time::timeout,
};
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

use crate::{
    pb::{vpn_command_response::Status, VpnCommandRequest, VpnCommandResponse},
    ProstStream, Socks5Config, TlsClientConnector, VpnError, YamuxCtrl,
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
        let (cmd, target_addr) = stream.request(self.config.dns_resolve).await?;
        debug!("request good");

        // TODO: 这里没有执行任何命令，后面想一下架构
        if !(stream.execute_command(cmd).await?) {
            warn!("Udp currently not supported");
            return Ok(());
        }

        let ca_cert = include_str!("../../fixtures/ca.cert");
        let connector = TlsClientConnector::new("kvserver.acme.inc", None, Some(ca_cert))?;

        // todo: loop get stream send to server
        let addr = "127.0.0.1:9527";
        let client_stream = TcpStream::connect(addr).await?;
        let client_stream = connector.connect(client_stream).await?;
        let mut ctrl = YamuxCtrl::new_client(client_stream, None);
        let client_stream = ctrl.open_stream().await?;

        let mut client = ProstClientStream::new(client_stream);
        let cmd = VpnCommandRequest::new_connect(uuid.clone(), target_addr);
        info!("Send a new cmd: {:?}", cmd);
        let res = client.execute_unary(&cmd).await?;
        info!("Got a new res: {:?}", res);

        if res.status() == Status::Success {
            // server connect target addr success
            stream
                .send_reply(0, SocketAddr::new([127, 0, 0, 1].into(), 0))
                .await?;
        } else {
            // server connect target addr failed
            stream.send_reply(1, "0.0.0.0:0".parse().unwrap()).await?;
            return Ok(());
        }

        let mut buf = vec![0u8; 1024 * 1000];
        loop {
            // if let Some(Ok(res)) = client.inner.next().await {
            //     info!("Got a new data: {:?}", res);
            //     if res.status() == Status::Success && res.data.len() > 0 {
            //         stream.write(&res.data).await.unwrap();
            //     }
            // }
            let res = client.inner.next();
            let res = match timeout(Duration::from_millis(20), res).await {
                Ok(result) => result,
                Err(_) => None,
            };
            if res.is_some() {
                let res = res.unwrap().unwrap();
                info!("Got a new data: {:?}", res);
                if res.status() == Status::Success && !res.data.is_empty() {
                    stream.write(&res.data).await.unwrap();
                }
            }

            // let n = stream.read(&mut buf).await?;
            // info!("get socks data size: {:?}", n);
            // if n == 0 {
            //     continue;
            // }

            // let cmd = VpnCommandRequest::new_data(uuid.clone(), buf[..n].to_vec());
            // info!("Send a new data: {:?}", cmd);
            // let ret = client.execute_unary(&cmd).await.unwrap();
            // info!("Got a new res: {:?}", ret);

            let res = stream.read(&mut buf);
            if let Ok(res) = timeout(Duration::from_millis(20), res).await {
                if res.is_ok() {
                    let n = res.unwrap();
                    info!("get socks data size: {:?}", n);
                    if n == 0 {
                        continue;
                    }
                    let cmd = VpnCommandRequest::new_data(uuid.clone(), buf[..n].to_vec());
                    info!("Send a new data: {:?}", cmd);
                    let ret = client.execute_unary(&cmd).await.unwrap();
                    info!("Got a new res: {:?}", ret);
                }
            }
        }

        #[allow(unreachable_code)]
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
