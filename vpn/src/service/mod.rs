pub mod client_read;
pub mod client_write;
mod server;

use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};

pub use client_read::*;
pub use client_write::*;

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
