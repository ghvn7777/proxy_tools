use futures::{channel::mpsc::Sender, SinkExt};

use crate::{Socks5ToClientMsg, VpnError};

#[allow(unused)]
pub struct TunnelWriter {
    pub id: u32,
    pub tx: Sender<Socks5ToClientMsg>,
}

impl TunnelWriter {
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
