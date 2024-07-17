use futures::{channel::mpsc::Sender, SinkExt};

use crate::{ServiceError, VpnError};

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
            .map_err(|_err| ServiceError::SendTunnelMsgError)?;
        Ok(())
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }
}
