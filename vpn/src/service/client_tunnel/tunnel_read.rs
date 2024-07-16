use futures::channel::mpsc::{Receiver, Sender};

use crate::{ClientToSocks5Msg, Socks5ToClientMsg};

#[allow(unused)]
pub struct TunnelReader {
    pub id: u32,
    pub tx: Sender<Socks5ToClientMsg>,
    pub rx: Option<Receiver<ClientToSocks5Msg>>,
}

impl TunnelReader {
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
