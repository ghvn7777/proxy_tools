mod vpn;

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
    //     pub fn new_get_data_stream(connect_id: String) -> Self {
    //         Self {
    //             connect_id,
    //             command: Some(vpn_command_request::Command::GetDatStream(true)),
    //         }
    //     }

    //     pub fn new_data(connect_id: String, data: Vec<u8>) -> Self {
    //         Self {
    //             connect_id,
    //             command: Some(vpn_command_request::Command::Data(data)),
    //         }
    //     }
}

// impl VpnCommandResponse {
//     pub fn new_error(msg: String) -> Self {
//         Self {
//             status: vpn_command_response::Status::Failed as i32,
//             message: msg,
//             data: Vec::new(),
//         }
//     }

//     pub fn new_success(msg: String) -> Self {
//         Self {
//             status: vpn_command_response::Status::Success as i32,
//             message: msg,
//             data: Vec::new(),
//         }
//     }

//     pub fn new_data(data: Vec<u8>) -> Self {
//         Self {
//             status: vpn_command_response::Status::Success as i32,
//             message: String::new(),
//             data,
//         }
//     }
// }
