use futures::channel::mpsc::Sender;

use crate::util::TargetAddr;

#[derive(Debug, Clone)]
pub enum Socks5ToClientMsg {
    Heartbeat,
    InitChannel(u32, Sender<ClientToSocks5Msg>),
    TcpConnect(u32, TargetAddr),
}

#[derive(Debug, Clone)]
pub enum ClientToSocks5Msg {
    Heartbeat,
}

#[derive(Clone)]
pub enum ClientChannelMsg {
    Connect(u32, Sender<ClientToSocks5Msg>),
}
