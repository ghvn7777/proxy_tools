use std::{borrow::BorrowMut, pin::Pin, sync::Arc};

use anyhow::Result;
use futures::Stream;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::Response;
use tracing::{debug, error, info, trace, warn};

use crate::{
    pb::{vpn_command_request::Command, VpnCommandRequest, VpnCommandResponse},
    util::TargetAddr,
    StreamMap, VpnError,
};

const CHANNEL_SIZE: usize = 1024;

pub type StreamingResponse = Pin<Box<dyn Stream<Item = Arc<VpnCommandResponse>> + Send>>;

type ServiceResult<T> = Result<Response<T>, VpnError>;
type ResponseStream = Pin<Box<dyn Stream<Item = VpnCommandResponse> + Send + 'static>>;

impl VpnCommandRequest {
    pub async fn execute(
        &self,
        streams: &StreamMap,
        connect_id: &String,
    ) -> Result<VpnCommandResponse, VpnError> {
        trace!("VpnCommandRequest: executing...");
        let res = match &self.command {
            Some(cmd) => match cmd {
                Command::Connect(addr) => {
                    let addr: TargetAddr = addr.try_into()?;
                    if let TargetAddr::Ip(addr) = addr.resolve_dns().await? {
                        debug!("get ip addr: {:?}", addr);

                        let client_stream = TcpStream::connect(addr).await?;
                        streams.insert(connect_id.clone(), client_stream);
                        info!("Connect to {:?} success", addr);

                        VpnCommandResponse::new_success("Connect success".to_string())
                    } else {
                        error!("Domain won't be resolved because `dns_resolve`'s config has been turned off.");
                        return Err(VpnError::TcpConnectkError(
                            "Domain won't be resolved".to_string(),
                        ));
                    }
                }
                Command::Disconnect(_) => {
                    if let Some((_, mut stream)) = streams.remove(connect_id) {
                        stream.shutdown().await?;
                        info!("Disconnect stream {:?}", connect_id);
                        VpnCommandResponse::new_success("Disconnect success".to_string())
                    } else {
                        warn!("Disconnect stream not found");
                        return Err(VpnError::TcpDisconnectError(
                            "Disconnect stream not found".to_string(),
                        ));
                    }
                }
                Command::Data(data) => match streams.get_mut(connect_id) {
                    Some(mut stream) => {
                        (*stream).borrow_mut().write_all(data).await?;
                        info!("Send data to {:?}", connect_id);
                        VpnCommandResponse::new_success("Send data success".to_string())
                    }
                    None => {
                        warn!("Send data stream not found");
                        VpnCommandResponse::new_error("Send data stream not found".to_string())
                    }
                },
                Command::GetDatStream(_) => {
                    return Err(VpnError::InternalError(
                        "GetDataStream Command error".to_string(),
                    ))
                }
            },
            None => return Err(VpnError::InternalError("Command error".to_string())),
        };

        // Ok(Box::pin(stream::once(async { Arc::new(res) })))
        Ok(res)
    }

    pub async fn get_real_streams(
        &self,
        streams: &StreamMap,
        connect_id: &str,
    ) -> ServiceResult<ResponseStream> {
        trace!("VpnCommandRequest: get_real_streams...");
        let (tx, rx) = mpsc::channel(CHANNEL_SIZE);

        let conntect_id_clone = connect_id.to_owned();
        let streams_clone = streams.clone();
        tokio::spawn(async move {
            if let Some(mut stream) = streams_clone.get_mut(&conntect_id_clone) {
                let mut buf = vec![0u8; 1024 * 1000]; // 1000 M
                loop {
                    let n = (*stream).borrow_mut().read(&mut buf).await?;
                    let ret = VpnCommandResponse::new_data(buf[..n].to_vec());
                    tx.send(ret).await?;
                }

                #[allow(unreachable_code)]
                Ok::<(), VpnError>(())
            } else {
                Err(VpnError::InternalError("Stream not found".to_string()))
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }
}
