mod client_read_stream;
mod client_write_stream;
mod server_read_stream;
mod server_write_stream;

use std::sync::Arc;

use client_read_stream::VpnClientProstReadStream;
use client_write_stream::VpnClientProstWriteStream;
use server_read_stream::VpnServerProstReadStream;
use server_write_stream::VpnServerProstWriteStream;

use tokio::net::TcpStream;

use crate::TextCrypt;

pub struct VpnClientStreamGenerator;

impl VpnClientStreamGenerator {
    pub fn generate(
        stream: TcpStream,
        crypt: Arc<Box<dyn TextCrypt>>,
    ) -> (VpnClientProstReadStream, VpnClientProstWriteStream) {
        let (r, w) = stream.into_split();
        let read_stream = VpnClientProstReadStream::new(r, crypt.clone());
        let write_stream = VpnClientProstWriteStream::new(w, crypt.clone());
        (read_stream, write_stream)
    }
}

pub struct VpnServerStreamGenerator;

impl VpnServerStreamGenerator {
    pub fn generate(
        stream: TcpStream,
        crypt: Arc<Box<dyn TextCrypt>>,
    ) -> (VpnServerProstReadStream, VpnServerProstWriteStream) {
        let (r, w) = stream.into_split();
        let read_stream = VpnServerProstReadStream::new(r, crypt.clone());
        let write_stream = VpnServerProstWriteStream::new(w, crypt.clone());
        (read_stream, write_stream)
    }
}
