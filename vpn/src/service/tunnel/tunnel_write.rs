use futures::{channel::mpsc::Sender, SinkExt};

use crate::VpnError;

#[allow(unused)]
pub struct TunnelWriter<T> {
    pub id: u32,
    pub tx: Sender<T>,
}

impl<T> TunnelWriter<T> {
    pub async fn send(&mut self, msg: T) -> Result<(), VpnError> {
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
