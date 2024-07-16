pub mod client;
mod client_tunnel;
mod intervals;
pub mod server;
mod streams;

use dashmap::DashMap;
use futures::channel::mpsc::Sender;

pub use client_tunnel::*;
pub use intervals::interval;
pub use streams::*;

use crate::ClientToSocks5Msg;

pub type ChannelMap = DashMap<u32, Sender<ClientToSocks5Msg>>;
pub const ALIVE_TIMEOUT_TIME_MS: u128 = 60000;
