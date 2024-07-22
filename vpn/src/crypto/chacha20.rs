use std::collections::HashMap;

use anyhow::Result;

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};

use crate::{process_genpass, DataCrypt};

pub struct EncryptChaCha20 {
    key: [u8; 32],
    nonce: [u8; 12],
}

impl DataCrypt for EncryptChaCha20 {
    fn encrypt(&self, buf: Vec<u8>) -> Result<Vec<u8>> {
        let cipher = ChaCha20Poly1305::new_from_slice(&self.key)
            .map_err(|_| anyhow::anyhow!("Failed to create ChaChaPoly1305 instance"))?;
        let nonce = Nonce::from_slice(&self.nonce);

        let ciphertext = cipher
            .encrypt(nonce, &buf[..])
            .map_err(|_| anyhow::anyhow!("Failed to encrypt data"))?;
        Ok(ciphertext)
    }

    fn decrypt(&self, buf: Vec<u8>) -> Result<Vec<u8>> {
        let cipher = ChaCha20Poly1305::new_from_slice(&self.key)
            .map_err(|_| anyhow::anyhow!("Failed to create ChaChaPoly1305 instance"))?;
        let nonce = Nonce::from_slice(&self.nonce);

        let plaintext = cipher
            .decrypt(nonce, &buf[..])
            .map_err(|_| anyhow::anyhow!("Failed to decrypt data"))?;
        Ok(plaintext)
    }
}

impl EncryptChaCha20 {
    pub fn new(key: [u8; 32], nonce: [u8; 12]) -> Self {
        Self { key, nonce }
    }

    pub fn try_new(key_and_nonce: impl AsRef<[u8]>) -> Result<Self> {
        let key_and_nonce = key_and_nonce.as_ref();
        let key = (&key_and_nonce[..32]).try_into()?;
        let nonce = (&key_and_nonce[32..32 + 12]).try_into()?;
        Ok(Self::new(key, nonce))
    }

    #[allow(unused)]
    fn generate() -> Result<HashMap<&'static str, Vec<u8>>> {
        let key = process_genpass(44, true, true, true, true)?;
        let mut map = HashMap::new();
        map.insert("chacha20.txt", key.as_bytes().to_vec());
        Ok(map)
    }
}
