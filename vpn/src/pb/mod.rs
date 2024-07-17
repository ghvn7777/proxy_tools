mod vpn;

use command_response::Response;
pub use vpn::*;

use crate::util::TargetAddr;

impl CommandRequest {
    pub fn new_tcp_connect(id: u32, target_addr: TargetAddr) -> Self {
        let tcp_connect = TcpConnect {
            destination: Some(target_addr.into()),
            id,
        };

        Self {
            command: Some(command_request::Command::TcpConnect(tcp_connect)),
        }
    }

    pub fn new_close_port(id: u32) -> Self {
        Self {
            command: Some(command_request::Command::ClosePort(id)),
        }
    }

    pub fn new_data(id: u32, data: Vec<u8>) -> Self {
        Self {
            command: Some(command_request::Command::Data(Data { id, data })),
        }
    }

    pub fn new_heartbeat() -> Self {
        Self {
            command: Some(command_request::Command::Heartbeat(
                Heartbeat::Pingpong as i32,
            )),
        }
    }
}

impl CommandResponse {
    pub fn new_data(id: u32, data: Vec<u8>) -> Self {
        Self {
            response: Some(Response::Data(Data { id, data })),
        }
    }

    pub fn new_tcp_connect_success(id: u32) -> Self {
        Self {
            response: Some(Response::TcpConnectSuccess(id)),
        }
    }

    pub fn new_tcp_connect_failed(id: u32) -> Self {
        Self {
            response: Some(Response::TcpConnectFailed(id)),
        }
    }

    pub fn new_close_port(id: u32) -> Self {
        Self {
            response: Some(Response::ClosePort(id)),
        }
    }

    pub fn new_heartbeat() -> Self {
        Self {
            response: Some(Response::Heartbeat(Heartbeat::Pingpong as i32)),
        }
    }
}
