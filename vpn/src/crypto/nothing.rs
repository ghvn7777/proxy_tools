use anyhow::Result;

use crate::DataCrypt;

pub struct Nothing;

impl DataCrypt for Nothing {
    fn encrypt(&self, buf: Vec<u8>) -> Result<Vec<u8>> {
        Ok(buf)
    }

    fn decrypt(&self, buf: Vec<u8>) -> Result<Vec<u8>> {
        Ok(buf)
    }
}
