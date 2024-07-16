mod client_read_stream;
mod client_write_stream;

use client_read_stream::VpnClientProstReadStream;
use client_write_stream::VpnClientProstWriteStream;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};

pub mod server_read_stream;

pub struct VpnClientStreamGenerator;

impl VpnClientStreamGenerator {
    pub fn generate(
        stream: TcpStream,
    ) -> (
        VpnClientProstReadStream<OwnedReadHalf>,
        VpnClientProstWriteStream<OwnedWriteHalf>,
    ) {
        let (r, w) = stream.into_split();
        let read_stream = VpnClientProstReadStream::new(r);
        let write_stream = VpnClientProstWriteStream::new(w);
        (read_stream, write_stream)
    }
}
