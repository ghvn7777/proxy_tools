use futures::{
    channel::mpsc::{Receiver, Sender},
    StreamExt,
};

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
