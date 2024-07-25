use std::{net::SocketAddr, path::Path, sync::Arc, time::Duration};

use anyhow::Result;
use futures::{channel::mpsc::Sender, Stream};
use s2n_quic::{client::Connect, Client};
use tokio_stream::StreamExt;
use tracing::{error, info, trace};

use crate::{
    client::run_client, interval, util::channel_bus, ClientMsg, DataCrypt, ServiceError, Tunnel,
    VpnError, HEARTBEAT_INTERVAL_MS,
};

pub struct S2nTunnel;

impl S2nTunnel {
    pub fn generate(server_addr: String, crypt: Arc<Box<dyn DataCrypt>>) -> Tunnel {
        let (main_sender, sub_senders, receivers) = channel_bus(10, 1000);
        let main_sender_clone = main_sender.clone();

        tokio::spawn(async move {
            let duration = Duration::from_millis(HEARTBEAT_INTERVAL_MS);
            let timer_stream = interval(duration, ClientMsg::Heartbeat);
            let mut msg_stream = timer_stream.merge(receivers);
            loop {
                match s2n_tunnel_core_task(
                    server_addr.clone(),
                    crypt.clone(),
                    &mut msg_stream,
                    main_sender_clone.clone(),
                )
                .await
                {
                    Ok(_) => info!("S2n tunnel core task finished"),
                    Err(e) => error!("S2n tunnel core task error: {:?}", e),
                }
            }
        });

        Tunnel {
            connect_id: 0,
            senders: sub_senders,
            main_sender,
        }
    }
}

async fn s2n_tunnel_core_task<S>(
    server_addr: String,
    crypt: Arc<Box<dyn DataCrypt>>,
    msg_stream: &mut S,
    main_sender_tx: Sender<ClientMsg>,
) -> Result<(), VpnError>
where
    S: Stream<Item = ClientMsg> + Unpin + Send + Sync + 'static,
{
    trace!("S2n tunnel core task start");
    let client = Client::builder()
        .with_tls(Path::new("cert.pem"))
        .map_err(|e| ServiceError::TlsError(e.to_string()))?
        .with_io("0.0.0.0:0")?
        .start()
        .map_err(|e| ServiceError::StartError(e.to_string()))?;

    let server_addr: SocketAddr = server_addr.parse().unwrap();
    let connect = Connect::new(server_addr).with_server_name("localhost");

    let mut connection = client
        .connect(connect)
        .await
        .map_err(|e| ServiceError::ConnectError(e.to_string()))?;

    // ensure the connection doesn't time out with inactivity
    connection
        .keep_alive(true)
        .map_err(|e| ServiceError::KeepAliveError(e.to_string()))?;
    let stream = connection
        .open_bidirectional_stream()
        .await
        .map_err(|e| ServiceError::ConnectTransformError(e.to_string()))?;

    run_client(stream, crypt, msg_stream, main_sender_tx).await?;
    info!("tunnel core task finished");

    info!("s2n tunnel core task finished");

    Ok(())
}
