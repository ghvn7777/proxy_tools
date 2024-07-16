mod socks_and_client;

use futures::channel::mpsc::Sender;

pub use socks_and_client::*;

pub enum TunnelPortMsg {
    ConnectOk(Vec<u8>),
    Data(Vec<u8>),
    ShutdownWrite,
    ClosePort,
}

#[derive(Clone)]
pub enum TunnelMsg {
    CSOpenPort(u32, Sender<TunnelPortMsg>),
    CSConnect(u32, Vec<u8>),
    CSConnectDN(u32, Vec<u8>, u16),
    CSUdpAssociate(u32, Vec<u8>),
    CSShutdownWrite(u32),
    CSClosePort(u32),
    CSData(u32, Vec<u8>),

    SCHeartbeat,
    SCClosePort(u32),
    SCShutdownWrite(u32),
    SCConnectOk(u32, Vec<u8>),
    SCData(u32, Vec<u8>),

    Heartbeat,
    TunnelPortHalfDrop(u32),
}
