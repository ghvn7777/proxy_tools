mod client_read_stream;
mod client_write_stream;
mod server_read_stream;
mod server_write_stream;

use client_read_stream::VpnClientProstReadStream;
use client_write_stream::VpnClientProstWriteStream;
use server_read_stream::VpnServerProstReadStream;
use server_write_stream::VpnServerProstWriteStream;

use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};

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

pub struct VpnServerStreamGenerator;

impl VpnServerStreamGenerator {
    pub fn generate(
        stream: TcpStream,
    ) -> (
        VpnServerProstReadStream<OwnedReadHalf>,
        VpnServerProstWriteStream<OwnedWriteHalf>,
    ) {
        let (r, w) = stream.into_split();
        let read_stream = VpnServerProstReadStream::new(r);
        let write_stream = VpnServerProstWriteStream::new(w);
        (read_stream, write_stream)
    }
}
