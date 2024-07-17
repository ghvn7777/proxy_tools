use std::{sync::Arc, time::Duration};

use futures::{channel::mpsc::Sender, join, Stream};
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tracing::{error, info};

use crate::{
    interval, util::channel_bus, ClientMsg, ClientPortMap, ServiceError, Tunnel,
    VpnClientStreamGenerator, VpnError, HEARTBEAT_INTERVAL_MS,
};

pub struct TcpTunnel;

impl TcpTunnel {
    pub fn generate(server_addr: String) -> Tunnel {
        let (main_sender, sub_senders, receivers) = channel_bus(10, 1000);
        let main_sender_clone = main_sender.clone();

        tokio::spawn(async move {
            let duration = Duration::from_millis(HEARTBEAT_INTERVAL_MS);
            let timer_stream = interval(duration, ClientMsg::Heartbeat);
            let mut msg_stream = timer_stream.merge(receivers);

            tcp_tunnel_core_task(
                server_addr.clone(),
                &mut msg_stream,
                main_sender_clone.clone(),
            )
            .await
            .expect("Tcp tunnel core task error");
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
) -> Result<(), VpnError> {
    let stream = match TcpStream::connect(&server_addr).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("TcpTunnel: connect to server failed: {:?}", e);
            return Err(ServiceError::TcpConnectError(server_addr).into());
        }
    };

    let port_map = Arc::new(ClientPortMap::new());
    // Split client to Server stream
    let (mut read_stream, mut write_stream) = VpnClientStreamGenerator::generate(stream);

    let r = async {
        read_stream
            .process(main_sender_tx, port_map.clone())
            .await
            .expect("tcp tunnel core task read stream error");
    };

    let w = async {
        write_stream
            .process(msg_stream, port_map.clone())
            .await
            .expect("tcp tunnel core task write stream error");
    };

    join!(r, w);

    port_map.clear();
    info!("Tcp tunnel core task finished");

    Ok(())
}
