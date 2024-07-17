use crate::util::TargetAddr;

#[derive(Debug, Clone)]
pub enum ServerMsg {
    Heartbeat,
    ServerToRemote(ServerToRemote),
    RemoteToServer(RemoteToServer),
}

#[derive(Debug, Clone)]
pub enum ServerToRemote {
    TcpConnect(u32, TargetAddr),
    ClosePort(u32),
    Data(u32, Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum RemoteToServer {
    TcpConnectSuccess(u32),
    TcpConnectFailed(u32),
    Data(u32, Vec<u8>),
    ClosePort(u32),
}

pub enum RemoteMsg {
    Data(Vec<u8>),
}

impl From<ServerToRemote> for ServerMsg {
    fn from(msg: ServerToRemote) -> Self {
        Self::ServerToRemote(msg)
    }
}

impl From<RemoteToServer> for ServerMsg {
    fn from(msg: RemoteToServer) -> Self {
        Self::RemoteToServer(msg)
    }
}
