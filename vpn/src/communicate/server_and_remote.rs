use crate::util::TargetAddr;

#[derive(Debug, Clone)]
pub enum ServerMsg {
    Heartbeat,
    TcpConnect(u32, TargetAddr),
    ClosePort(u32),
}
