mod client_read_stream;
mod client_write_stream;
mod quic_client_read_stream;
mod quic_client_write_stream;
mod quic_server_read_stream;
mod quic_server_write_stream;
mod server_read_stream;
mod server_write_stream;

use std::sync::Arc;

use client_read_stream::TcpClientProstReadStream;
use client_write_stream::TcpClientProstWriteStream;
use quic_client_read_stream::QuicClientProstReadStream;
use quic_client_write_stream::QuicClientProstWriteStream;
use quic_server_read_stream::QuicServerProstReadStream;
use quic_server_write_stream::QuicServerProstWriteStream;
use quinn::Connection;
use server_read_stream::TcpServerProstReadStream;
use server_write_stream::TcpServerProstWriteStream;

use anyhow::Result;
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

pub struct QuicServerStreamGenerator;

impl QuicServerStreamGenerator {
    pub async fn generate(
        conn: Connection,
        crypt: Arc<Box<dyn DataCrypt>>,
    ) -> Result<(QuicServerProstReadStream, QuicServerProstWriteStream)> {
        let (send, recv) = conn.accept_bi().await?;
        let read_stream = QuicServerProstReadStream::new(recv, crypt.clone());
        let write_stream = QuicServerProstWriteStream::new(send, crypt.clone());
        Ok((read_stream, write_stream))
    }
}

pub struct QuicClientStreamGenerator;
impl QuicClientStreamGenerator {
    pub async fn generate(
        conn: Connection,
        crypt: Arc<Box<dyn DataCrypt>>,
    ) -> Result<(QuicClientProstReadStream, QuicClientProstWriteStream)> {
        let (send, recv) = conn.open_bi().await?;
        let read_stream = QuicClientProstReadStream::new(recv, crypt.clone());
        let write_stream = QuicClientProstWriteStream::new(send, crypt.clone());
        Ok((read_stream, write_stream))
    }
}
