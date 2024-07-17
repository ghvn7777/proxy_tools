use futures::channel::mpsc::Sender;

use crate::util::TargetAddr;

#[derive(Debug, Clone)]
pub enum Socks5ToClientMsg {
    InitChannel(u32, Sender<ClientToSocks5Msg>),
    TcpConnect(u32, TargetAddr),
    ClosePort(u32),
}

#[derive(Debug, Clone)]
pub enum ClientToSocks5Msg {
    Data(u32, Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum ClientMsg {
    Heartbeat,
    Socks5ToClient(Socks5ToClientMsg),
    ClientToSocks5(ClientToSocks5Msg),
}

impl From<Socks5ToClientMsg> for ClientMsg {
    fn from(msg: Socks5ToClientMsg) -> Self {
        Self::Socks5ToClient(msg)
    }
}

impl From<ClientToSocks5Msg> for ClientMsg {
    fn from(msg: ClientToSocks5Msg) -> Self {
        Self::ClientToSocks5(msg)
    }
}
