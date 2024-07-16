use anyhow::Result;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::{join, SinkExt, Stream};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tracing::info;

use crate::util::{channel_bus, SubSenders};
use crate::{
    interval, ChannelMap, ClientToSocks5Msg, Socks5ToClientMsg, VpnError, VpnProstClientStream,
};

pub const HEARTBEAT_INTERVAL_MS: u64 = 5000;

#[allow(unused)]
pub struct TunnelWritePort {
    id: u32,
    tx: Sender<Socks5ToClientMsg>,
}

#[allow(unused)]
pub struct TunnelReadPort {
    id: u32,
    tx: Sender<Socks5ToClientMsg>,
    rx: Option<Receiver<ClientToSocks5Msg>>,
}

pub struct Tunnel {
    connect_id: u32,
    senders: SubSenders<Socks5ToClientMsg>,
    main_sender: Sender<Socks5ToClientMsg>,
}

impl TunnelWritePort {
    pub async fn send(&mut self, msg: Socks5ToClientMsg) -> Result<(), VpnError> {
        self.tx
            .send(msg)
            .await
            .expect("Send tunnel write port failed");
        Ok(())
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }
}

impl TunnelReadPort {
    // pub async fn recv(&mut self) -> Option<ClientToSocks5Msg> {
    //     self.rx.as_mut().unwrap().next().await
    // }

    // pub async fn send(&mut self, msg: Socks5ToClientMsg) -> Result<(), VpnError> {
    //     self.tx
    //         .send(msg)
    //         .await
    //         .expect("Send tunnel read port failed");
    //     Ok(())
    // }

    pub fn get_id(&self) -> u32 {
        self.id
    }
}

impl Tunnel {
    pub async fn generate(&mut self) -> Result<(TunnelWritePort, TunnelReadPort), VpnError> {
        let connect_id = self.connect_id;
        self.connect_id += 1;

        let (tx, rx) = channel(1000);

        self.main_sender
            .send(Socks5ToClientMsg::InitChannel(connect_id, tx))
            .await
            .expect("Send init channel failed");

        let sender = self.senders.get_one_sender();

        Ok((
            TunnelWritePort {
                id: connect_id,
                tx: sender.clone(),
            },
            TunnelReadPort {
                id: connect_id,
                tx: sender.clone(),
                rx: Some(rx),
            },
        ))
    }
}

pub struct TcpTunnel;

impl TcpTunnel {
    pub fn generate(server_addr: String) -> Tunnel {
        let (main_sender, sub_senders, receivers) = channel_bus(10, 1000);
        let main_sender_clone = main_sender.clone();

        tokio::spawn(async move {
            let duration = Duration::from_millis(HEARTBEAT_INTERVAL_MS);
            let timer_stream = interval(duration, Socks5ToClientMsg::Heartbeat);
            let mut msg_stream = timer_stream.merge(receivers);
            loop {
                tcp_tunnel_core_task(
                    server_addr.clone(),
                    &mut msg_stream,
                    main_sender_clone.clone(),
                )
                .await;
            }
        });

        Tunnel {
            connect_id: 0,
            senders: sub_senders,
            main_sender,
        }
    }
}

async fn tcp_tunnel_core_task<S: Stream<Item = Socks5ToClientMsg> + Unpin>(
    server_addr: String,
    msg_stream: &mut S,
    main_sender_tx: Sender<Socks5ToClientMsg>,
) {
    let stream = match TcpStream::connect(&server_addr).await {
        Ok(stream) => stream,
        Err(e) => {
            info!("TcpTunnel: connect to server failed: {:?}", e);
            return;
        }
    };

    let channel_map = Arc::new(ChannelMap::new());
    // Split client to Server stream
    let (mut read_stream, mut write_stream) = VpnProstClientStream::generate(stream);

    let r = async {
        read_stream
            .process(main_sender_tx, channel_map.clone())
            .await
            .expect("tcp tunnel core task read stream error");
    };

    let w = async {
        write_stream
            .process(msg_stream, channel_map.clone())
            .await
            .expect("tcp tunnel core task write stream error");
    };

    join!(r, w);
    channel_map.clear();
}
