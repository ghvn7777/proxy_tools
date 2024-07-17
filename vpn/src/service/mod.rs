pub mod client;
mod intervals;
pub mod server;
mod streams;
mod tunnel;

use std::collections::HashMap;

use futures::channel::mpsc::Sender;

pub use intervals::interval;
pub use streams::*;
pub use tunnel::*;

use crate::ClientToSocks5Msg;

pub type ClientPortMap = HashMap<u32, Sender<ClientToSocks5Msg>>;
pub const ALIVE_TIMEOUT_TIME_MS: u64 = 60000;
pub const HEARTBEAT_INTERVAL_MS: u64 = 5000;
