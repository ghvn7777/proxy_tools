use futures::{
    channel::mpsc::{Receiver, Sender},
    SinkExt, StreamExt,
};

use crate::{ServiceError, VpnError};

#[allow(unused)]
pub struct TunnelReader<T, U> {
    pub id: u32,
    pub tx: Sender<T>,
    pub rx: Option<Receiver<U>>,
}

impl<T, U> TunnelReader<T, U> {
    pub async fn read(&mut self) -> Option<U> {
        match self.rx {
            Some(ref mut receiver) => receiver.next().await,
            None => None,
        }
    }

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
