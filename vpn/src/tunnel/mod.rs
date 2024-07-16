mod intervals;
mod porthub;
mod tcp_tunnel;

use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::stream::SelectAll;
use futures::SinkExt;

pub use porthub::*;
pub use tcp_tunnel::TcpTunnel;

use crate::{TunnelMsg, TunnelPortMsg};

pub type Receivers<T> = SelectAll<Receiver<T>>;
pub type MainSender<T> = Sender<T>;
pub struct SubSenders<T>(Vec<Sender<T>>, usize);

#[allow(unused)]
pub struct TunnelWritePort {
    id: u32,
    tx: Sender<TunnelMsg>,
}

#[allow(unused)]
pub struct TunnelReadPort {
    id: u32,
    tx: Sender<TunnelMsg>,
    rx: Option<Receiver<TunnelPortMsg>>,
}

pub struct Tunnel {
    id: u32,
    senders: SubSenders<TunnelMsg>,
    main_sender: MainSender<TunnelMsg>,
}

impl Tunnel {
    pub async fn open_port(&mut self) -> (TunnelWritePort, TunnelReadPort) {
        let id = self.id;
        self.id += 1;

        let (tx, rx) = channel(1000);
        let _ = self.main_sender.send(TunnelMsg::CSOpenPort(id, tx)).await;

        let sender = self.senders.get_one_sender();

        (
            TunnelWritePort {
                id,
                tx: sender.clone(),
            },
            TunnelReadPort {
                id,
                tx: sender.clone(),
                rx: Some(rx),
            },
        )
    }
}

impl<T> SubSenders<T> {
    pub fn get_one_sender(&mut self) -> Sender<T> {
        self.1 = (self.1 + 1) % self.0.len();

        self.0.get(self.1).unwrap().clone()
    }
}

pub fn channel_bus<T>(
    bus_num: usize,
    buffer: usize,
) -> (MainSender<T>, SubSenders<T>, Receivers<T>) {
    let (main_sender, main_receiver) = channel(buffer);
    let mut receivers = Receivers::new();
    let mut sub_senders = SubSenders(Vec::new(), 0);

    receivers.push(main_receiver);
    for _ in 0..bus_num {
        let (sender, receiver) = channel(buffer);
        sub_senders.0.push(sender);
        receivers.push(receiver);
    }

    (main_sender, sub_senders, receivers)
}
