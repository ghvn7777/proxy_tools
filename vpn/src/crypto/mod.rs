mod chacha20;
mod gen_pass;
mod nothing;

use std::sync::Arc;

use anyhow::Result;

pub use chacha20::EncryptChaCha20;
pub use gen_pass::process_genpass;
pub use nothing::Nothing;
use tracing::info;

use crate::util::get_content;

pub trait DataCrypt: Sync + Send + 'static {
    /// Encrypt the data from the reader and return the ciphertext
    fn encrypt(&self, buf: &[u8]) -> Result<Vec<u8>>;

    /// Decrypt the data from the reader and return the plaintext
    fn decrypt(&self, buf: &[u8]) -> Result<Vec<u8>>;
}

pub fn get_crypt(file_path: &Option<String>) -> Result<Arc<Box<dyn DataCrypt>>> {
    let crypt = if file_path.is_some() {
        let key = file_path.as_ref().unwrap();
        let key = get_content(key)?;
        let crypt: Box<dyn DataCrypt> = Box::new(EncryptChaCha20::try_new(key)?);
        info!("Using chacha20 crypt");
        crypt
    } else {
        let crypt: Box<dyn DataCrypt> = Box::new(Nothing);
        info!("Nothing crypt");
        crypt
    };

    Ok(Arc::new(crypt))
}
