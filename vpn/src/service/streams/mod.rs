mod read_stream;
mod write_stream;

use quinn::{Connection, RecvStream, SendStream};
pub use read_stream::ProstReadStream;
use tokio::{
    io::{AsyncReadExt, AsyncWrite},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};
pub use write_stream::ProstWriteStream;

use anyhow::Result;
use tonic::async_trait;

use crate::VpnError;

/// server quic connection, use accept_bi to get recv and send stream
pub struct ServerQuicConn(Connection);
/// client quic connection, use open_bi to get recv and send stream
pub struct ClientQuicConn(Connection);

#[async_trait]
pub trait ReadStream<M>: Send + Sync + 'static {
    async fn next(&mut self) -> Result<M, VpnError>;
}

#[async_trait]
pub trait WriteStream<M>: Send + Sync + 'static {
    async fn send(&mut self, msg: &M) -> Result<(), VpnError>;
}

#[async_trait]
pub trait StreamSplit
where
    Self::ReadStream: AsyncReadExt + Unpin + Send + Sync + 'static,
    Self::WriteStream: AsyncWrite + Unpin + Send + Sync + 'static,
{
    type ReadStream;
    type WriteStream;

    async fn stream_split(self) -> (Self::ReadStream, Self::WriteStream);
}

#[async_trait]
impl StreamSplit for TcpStream {
    type ReadStream = OwnedReadHalf;
    type WriteStream = OwnedWriteHalf;
    async fn stream_split(self) -> (Self::ReadStream, Self::WriteStream) {
        self.into_split()
    }
}

#[async_trait]
impl StreamSplit for ServerQuicConn {
    type ReadStream = RecvStream;
    type WriteStream = SendStream;
    async fn stream_split(self) -> (Self::ReadStream, Self::WriteStream) {
        let res = self.0.accept_bi().await.unwrap();
        (res.1, res.0)
    }
}

#[async_trait]
impl StreamSplit for ClientQuicConn {
    type ReadStream = RecvStream;
    type WriteStream = SendStream;
    async fn stream_split(self) -> (Self::ReadStream, Self::WriteStream) {
        let res = self.0.open_bi().await.unwrap();
        (res.1, res.0)
    }
}

impl ServerQuicConn {
    pub fn new(conn: Connection) -> Self {
        Self(conn)
    }
}

impl ClientQuicConn {
    pub fn new(conn: Connection) -> Self {
        Self(conn)
    }
}
