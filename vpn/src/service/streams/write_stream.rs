use std::marker::PhantomData;
use std::sync::Arc;

use prost::Message;
use tokio::io::AsyncWriteExt;
use tonic::async_trait;

use crate::DataCrypt;
use crate::VpnError;

use super::WriteStream;

pub struct ProstWriteStream<M, T> {
    inner: T,
    crypt: Arc<Box<dyn DataCrypt>>,
    _phantom: PhantomData<M>,
}

impl<M, T> ProstWriteStream<M, T> {
    pub fn new(stream: T, crypt: Arc<Box<dyn DataCrypt>>) -> Self {
        Self {
            inner: stream,
            crypt,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<M, T> WriteStream<M> for ProstWriteStream<M, T>
where
    M: Message + Send + 'static,
    T: AsyncWriteExt + Unpin + Send + Sync + 'static,
{
    async fn send(&mut self, msg: &M) -> Result<(), VpnError> {
        let mut buf = Vec::new();
        msg.encode(&mut buf)?;

        // info!("before encrypt buf len: {:?}", buf.len());
        let buf = self.crypt.encrypt(buf)?;
        // info!("after encrypt buf len: {:?}", buf.len());

        self.inner.write_i32(buf.len() as i32).await?;
        self.inner.write_all(&buf).await?;
        self.inner.flush().await?;
        Ok(())
    }
}
