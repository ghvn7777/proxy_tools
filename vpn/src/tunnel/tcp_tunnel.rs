use async_std::{prelude::FutureExt, stream::StreamExt};
use futures::{channel::mpsc::Sender, Stream};
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::info;

use crate::VpnProstClientStream;

use super::{channel_bus, intervals::interval, PortHub, Tunnel, TunnelMsg};

pub const HEARTBEAT_INTERVAL_MS: u64 = 5000;

pub struct TcpTunnel;

impl TcpTunnel {
    pub fn generate(tid: u32, server_addr: String) -> Tunnel {
        let (main_sender, sub_senders, receivers) = channel_bus(10, 1000);
        let core_sender = main_sender.clone();

        tokio::spawn(async move {
            let duration = Duration::from_millis(HEARTBEAT_INTERVAL_MS);
            let timer_stream = interval(duration, TunnelMsg::Heartbeat);
            let mut msg_stream = timer_stream.merge(receivers);
            loop {
                tcp_tunnel_core_task(
                    tid,
                    server_addr.clone(),
                    &mut msg_stream,
                    core_sender.clone(),
                )
                .await;
            }
        });

        Tunnel {
            id: 0,
            senders: sub_senders,
            main_sender,
        }
    }
}

async fn tcp_tunnel_core_task<S: Stream<Item = TunnelMsg> + Unpin>(
    tid: u32,
    server_addr: String,
    msg_stream: &mut S,
    core_tx: Sender<TunnelMsg>,
) {
    let stream = match TcpStream::connect(&server_addr).await {
        Ok(stream) => stream,
        Err(e) => {
            info!("TcpTunnel: connect to server failed: {:?}", e);
            return;
        }
    };

    let port_hub = PortHub::new(tid);

    let (mut read_stream, mut write_stream) = VpnProstClientStream::generate(stream);

    let r = async {
        read_stream
            .process(core_tx)
            .await
            .expect("tcp tunnel core task read stream error");
    };

    let w = async {
        write_stream
            .process(msg_stream, port_hub)
            .await
            .expect("tcp tunnel core task write stream error");
    };

    r.join(w).await;
}
