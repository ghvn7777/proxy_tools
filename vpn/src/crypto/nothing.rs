use anyhow::Result;

use crate::TextCrypt;

pub struct Nothing;

impl TextCrypt for Nothing {
    fn encrypt(&self, buf: &[u8]) -> Result<Vec<u8>> {
        Ok(buf.to_vec())
    }

    fn decrypt(&self, buf: &[u8]) -> Result<Vec<u8>> {
        Ok(buf.to_vec())
    }
}
