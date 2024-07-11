mod socks5_stream;

use std::sync::Arc;

pub use socks5_stream::Socks5Stream;

use tokio::io::{AsyncRead, AsyncWrite};
use tracing::trace;

use crate::{Socks5Config, VpnError};

pub struct Socks5ServerStream<S> {
    inner: Socks5Stream<S>,
    config: Arc<Socks5Config>,
}

impl<S> Socks5ServerStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(stream: S, config: Arc<Socks5Config>) -> Self {
        Self {
            inner: Socks5Stream::new(stream),
            config,
        }
    }

    pub async fn process(self) -> Result<(), VpnError> {
        trace!("Socks5ServerStream: processing...");
        let mut stream = self.inner;
        stream.handshake(&self.config.auth_type).await?;
        stream.request(self.config.dns_resolve).await?;
        Ok(())
    }
}
