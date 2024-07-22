pub mod client;
mod intervals;
mod processor;
pub mod server;
mod streams;
mod tunnel;

use std::collections::HashMap;

use futures::channel::mpsc::Sender;

pub use intervals::interval;
pub use processor::*;
pub use streams::*;
pub use tunnel::*;

use crate::SocksMsg;

pub type ClientPortMap = HashMap<u32, Sender<SocksMsg>>;
pub const ALIVE_TIMEOUT_TIME_MS: u64 = 60000;
pub const HEARTBEAT_INTERVAL_MS: u64 = 5000;
