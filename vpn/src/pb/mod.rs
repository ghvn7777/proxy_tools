mod vpn;

pub use vpn::*;

use crate::util::TargetAddr;

impl VpnCommandRequest {
    pub fn new_connect(connect_id: String, target_addr: TargetAddr) -> Self {
        Self {
            connect_id,
            command: Some(vpn_command_request::Command::Connect(target_addr.into())),
        }
    }
}

impl VpnCommandResponse {
    pub fn new_error(msg: String) -> Self {
        Self {
            status: vpn_command_response::Status::Failed as i32,
            message: msg,
            data: Vec::new(),
        }
    }
}
