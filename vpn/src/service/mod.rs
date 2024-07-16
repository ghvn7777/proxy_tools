mod client;
mod intervals;
pub mod read_stream;
mod server;
pub mod write_stream;

use dashmap::DashMap;
use futures::channel::mpsc::Sender;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};

pub use client::*;
pub use intervals::interval;
pub use read_stream::*;
pub use write_stream::*;

use crate::ClientToSocks5Msg;

pub type ChannelMap = DashMap<u32, Sender<ClientToSocks5Msg>>;
pub const ALIVE_TIMEOUT_TIME_MS: u128 = 60000;

pub struct VpnProstClientStream;

impl VpnProstClientStream {
    pub fn generate(
        stream: TcpStream,
    ) -> (
        VpnProstReadStream<OwnedReadHalf>,
        VpnProstWriteStream<OwnedWriteHalf>,
    ) {
        let (r, w) = stream.into_split();
        let read_stream = VpnProstReadStream::new(r);
        let write_stream = VpnProstWriteStream::new(w);
        (read_stream, write_stream)
    }
}
