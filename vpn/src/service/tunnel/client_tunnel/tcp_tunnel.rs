use std::{sync::Arc, time::Duration};

use futures::{channel::mpsc::Sender, Stream};
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tracing::{error, info, trace};

use crate::{
    interval, util::channel_bus, ClientMsg, DataCrypt, ServiceError, Tunnel,
    VpnClientStreamGenerator, VpnError, HEARTBEAT_INTERVAL_MS,
};

pub struct TcpTunnel;

impl TcpTunnel {
    pub fn generate(server_addr: String, crypt: Arc<Box<dyn DataCrypt>>) -> Tunnel {
        let (main_sender, sub_senders, receivers) = channel_bus(10, 1000);
        let main_sender_clone = main_sender.clone();

        tokio::spawn(async move {
            let duration = Duration::from_millis(HEARTBEAT_INTERVAL_MS);
            let timer_stream = interval(duration, ClientMsg::Heartbeat);
            let mut msg_stream = timer_stream.merge(receivers);
            loop {
                match tcp_tunnel_core_task(
                    server_addr.clone(),
                    &mut msg_stream,
                    main_sender_clone.clone(),
                    crypt.clone(),
                )
                .await
                {
                    Ok(_) => info!("Tcp tunnel core task finished"),
                    Err(e) => error!("Tcp tunnel core task error: {:?}", e),
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

async fn tcp_tunnel_core_task<S: Stream<Item = ClientMsg> + Unpin>(
    server_addr: String,
    msg_stream: &mut S,
    main_sender_tx: Sender<ClientMsg>,
    crypt: Arc<Box<dyn DataCrypt>>,
) -> Result<(), VpnError> {
    trace!("Tcp tunnel core task start");
    let stream = match TcpStream::connect(&server_addr).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("TcpTunnel: connect to server failed: {:?}", e);
            tokio::time::sleep(Duration::from_millis(6000)).await;
            return Err(ServiceError::TcpConnectError(server_addr).into());
        }
    };

    // Split client to Server stream
    let (mut read_stream, mut write_stream) = VpnClientStreamGenerator::generate(stream, crypt);

    let r = async {
        match read_stream.process(main_sender_tx).await {
            Ok(_) => info!("Tcp tunnel core task read stream finished"),
            Err(e) => error!("Tcp tunnel core task read stream error: {:?}", e),
        }
    };

    let w = async {
        match write_stream.process(msg_stream).await {
            Ok(_) => info!("Tcp tunnel core task write stream finished"),
            Err(e) => error!("Tcp tunnel core task write stream error: {:?}", e),
        }
    };

    // join!(r, w);
    tokio::select! {
        _ = r => {
            info!("Tcp tunnel core task read stream end");
        }
        _ = w => {
            info!("Tcp tunnel core task write stream end");
        }
    };

    info!("Tcp tunnel core task finished");

    Ok(())
}
