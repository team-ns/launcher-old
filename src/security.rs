use std::fs::{File, OpenOptions};
use std::io::{prelude::*, Result};
use std::path::Path;
use openssl::rsa::{Rsa};
use openssl::pkey::{Public, Private};
use futures::AsyncWriteExt;
use log::info;

pub fn get_manager() -> Result<SecurityManager> {
    let public_key = Path::new("public_key");
    let private_key = Path::new("private_key");
    if !public_key.exists() || !private_key.exists() {
        info!("Creating new RSA keys...");
        SecurityManager::create_keys(public_key, private_key);
    }
    let mut public_buf = vec![0u8;2048];
    let mut private_buf = vec![0u8;2048];
    File::open(public_key)?.read_exact(&mut public_buf);
    File::open(private_key)?.read_exact(&mut private_buf);
    Ok(SecurityManager {
        public_key: Rsa::public_key_from_der(&public_buf)?,
        private_key: Rsa::private_key_from_der(&private_buf)?,
    })
}

pub struct SecurityManager {
    pub public_key: Rsa<Public>,
    pub private_key: Rsa<Private>,
}

impl SecurityManager {
    fn create_keys(public_key: &Path, private_key: &Path) -> Result<()> {
        let rsa = Rsa::generate(2048)?;
        SecurityManager::create_key(public_key, &rsa.public_key_to_der()?)?;
        SecurityManager::create_key(private_key, &rsa.private_key_to_der()?)?;
        Ok(())
    }

    fn create_key(path: &Path, buf: &[u8]) -> Result<()> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)?.write_all(buf)
    }
}