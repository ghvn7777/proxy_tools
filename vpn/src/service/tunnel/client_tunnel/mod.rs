mod tcp_tunnel;

use crate::{
    util::SubSenders, ClientToSocks5Msg, Socks5ToClientMsg, TunnelReader, TunnelWriter, VpnError,
};

use futures::{
    channel::mpsc::{channel, Sender},
    SinkExt,
};
pub use tcp_tunnel::*;

pub struct Tunnel {
    connect_id: u32,
    senders: SubSenders<Socks5ToClientMsg>,
    main_sender: Sender<Socks5ToClientMsg>,
}

impl Tunnel {
    pub async fn generate(
        &mut self,
    ) -> Result<
        (
            TunnelWriter<Socks5ToClientMsg>,
            TunnelReader<Socks5ToClientMsg, ClientToSocks5Msg>,
        ),
        VpnError,
    > {
        let connect_id = self.connect_id;
        self.connect_id += 1;

        let (tx, rx) = channel(1000);

        self.main_sender
            .send(Socks5ToClientMsg::InitChannel(connect_id, tx))
            .await
            .expect("Send init channel failed");

        let sender = self.senders.get_one_sender();

        Ok((
            TunnelWriter {
                id: connect_id,
                tx: sender.clone(),
            },
            TunnelReader {
                id: connect_id,
                tx: sender.clone(),
                rx: Some(rx),
            },
        ))
    }
}
