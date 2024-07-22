mod client_read_stream;
mod client_write_stream;
mod server_read_stream;
mod server_write_stream;

use std::sync::Arc;

use client_read_stream::TcpClientProstReadStream;
use client_write_stream::TcpClientProstWriteStream;
use server_read_stream::TcpServerProstReadStream;
use server_write_stream::TcpServerProstWriteStream;

use tokio::net::TcpStream;

use crate::DataCrypt;

pub struct TcpClientStreamGenerator;

impl TcpClientStreamGenerator {
    pub fn generate(
        stream: TcpStream,
        crypt: Arc<Box<dyn DataCrypt>>,
    ) -> (TcpClientProstReadStream, TcpClientProstWriteStream) {
        let (r, w) = stream.into_split();
        let read_stream = TcpClientProstReadStream::new(r, crypt.clone());
        let write_stream = TcpClientProstWriteStream::new(w, crypt.clone());
        (read_stream, write_stream)
    }
}

pub struct TcpServerStreamGenerator;

impl TcpServerStreamGenerator {
    pub fn generate(
        stream: TcpStream,
        crypt: Arc<Box<dyn DataCrypt>>,
    ) -> (TcpServerProstReadStream, TcpServerProstWriteStream) {
        let (r, w) = stream.into_split();
        let read_stream = TcpServerProstReadStream::new(r, crypt.clone());
        let write_stream = TcpServerProstWriteStream::new(w, crypt.clone());
        (read_stream, write_stream)
    }
}
