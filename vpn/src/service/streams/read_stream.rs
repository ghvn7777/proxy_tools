use std::{marker::PhantomData, sync::Arc};

use anyhow::Result;

use prost::Message;
use tokio::io::AsyncReadExt;
use tonic::async_trait;
use tracing::error;

use crate::{DataCrypt, ServiceError, VpnError};

use super::ReadStream;

pub struct ProstReadStream<M, T> {
    pub inner: T,
    pub crypt: Arc<Box<dyn DataCrypt>>,
    _phantom: PhantomData<M>,
}

impl<M, T> ProstReadStream<M, T> {
    pub fn new(stream: T, crypt: Arc<Box<dyn DataCrypt>>) -> Self {
        Self {
            inner: stream,
            crypt,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<M, T> ReadStream<M> for ProstReadStream<M, T>
where
    T: AsyncReadExt + Unpin + Send + Sync + 'static,
    M: Message + Default + Send + 'static,
{
    async fn next(&mut self) -> Result<M, VpnError> {
        let len = self.inner.read_i32().await? as usize;
        let mut buf = vec![0; len];
        self.inner.read_exact(&mut buf).await?;

        // info!("before decrypt buf: {:?}", buf);
        let Ok(buf) = self.crypt.as_ref().decrypt(&buf) else {
            error!("decrypt error (maybe the crypt key is wrong)");
            return Err(ServiceError::DecryptError.into());
        };
        // info!("after decrypt buf: {:?}", buf);

        let msg: M = match Message::decode(&buf[..]) {
            Ok(msg) => msg,
            Err(e) => {
                error!("decode error (maybe the crypt key is wrong): {:?}", e);
                return Err(e.into());
            }
        };
        Ok(msg)
    }
}
