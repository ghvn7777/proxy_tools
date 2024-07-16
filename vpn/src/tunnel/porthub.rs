use std::collections::HashMap;

use futures::{channel::mpsc::Sender, SinkExt};
use tracing::{error, info};

use super::TunnelPortMsg;

pub struct Port {
    pub address: String,
    pub count: u32,
    pub tx: Sender<TunnelPortMsg>,
}

pub struct PortHub(u32, HashMap<u32, Port>);

impl PortHub {
    pub fn new(id: u32) -> Self {
        PortHub(id, HashMap::new())
    }

    pub fn get_id(&self) -> u32 {
        self.0
    }

    pub fn add_port(&mut self, id: u32, tx: Sender<TunnelPortMsg>) {
        self.1.insert(
            id,
            Port {
                address: String::new(),
                count: 2,
                tx,
            },
        );
    }

    pub fn update_address(&mut self, id: u32, address: String) {
        if let Some(value) = self.1.get_mut(&id) {
            value.address = address;
        }
    }

    pub fn drop_port_half(&mut self, id: u32) {
        let self_id = self.get_id();

        if let Some(value) = self.1.get_mut(&id) {
            value.count -= 1;
            if value.count == 0 {
                info!("{}.{}: drop tunnel port {}", self_id, id, value.address);
                self.1.remove(&id);
            }
        } else {
            info!("{}.{}: drop unknown tunnel port", self.get_id(), id);
        }
    }

    pub fn clear_ports(&mut self) {
        self.1.clear();
    }

    pub fn client_close_port(&mut self, id: u32) {
        match self.1.get(&id) {
            Some(value) => {
                info!("{}.{}: client close {}", self.get_id(), id, value.address);
                self.1.remove(&id);
            }

            None => {
                info!("{}.{}: client close unknown server", self.get_id(), id);
            }
        }
    }

    pub fn server_close_port(&mut self, id: u32) {
        match self.1.get(&id) {
            Some(value) => {
                info!("{}.{}: server close {}", self.get_id(), id, value.address);
                self.1.remove(&id);
            }

            None => {
                info!("{}.{}: server close unknown client", self.get_id(), id);
            }
        }
    }

    pub fn client_shutdown(&self, id: u32) {
        match self.1.get(&id) {
            Some(value) => {
                info!(
                    "{}.{}: client shutdown {}",
                    self.get_id(),
                    id,
                    value.address
                );
            }

            None => {
                info!("{}.{}: client shutdown unknown server", self.get_id(), id);
            }
        }
    }

    pub async fn server_shutdown(&mut self, id: u32) {
        match self.1.get(&id) {
            Some(value) => {
                info!(
                    "{}.{}: server shutdown write {}",
                    self.get_id(),
                    id,
                    value.address
                );
                self.try_send_msg(id, TunnelPortMsg::ShutdownWrite).await;
            }

            None => {
                info!(
                    "{}.{}: server shutdown write unknown client",
                    self.get_id(),
                    id
                );
            }
        }
    }

    pub async fn connect_ok(&mut self, id: u32, buf: Vec<u8>) {
        match self.1.get(&id) {
            Some(value) => {
                info!("{}.{}: connect {} ok", self.get_id(), id, value.address);
                self.try_send_msg(id, TunnelPortMsg::ConnectOk(buf)).await;
            }

            None => {
                info!("{}.{}: connect unknown server ok", self.get_id(), id);
            }
        }
    }

    pub async fn server_send_data(&mut self, id: u32, buf: Vec<u8>) {
        self.try_send_msg(id, TunnelPortMsg::Data(buf)).await;
    }

    pub async fn try_send_msg(&mut self, id: u32, msg: TunnelPortMsg) {
        let self_id = self.get_id();

        if let Some(value) = self.1.get_mut(&id) {
            match value.tx.send(msg).await {
                Ok(_) => {}
                Err(err) => {
                    error!(
                        "{}.{}: send msg to the channel of {} error: {}",
                        self_id, id, value.address, err
                    );
                    self.1.remove(&id);
                }
            }
        }
    }
}
