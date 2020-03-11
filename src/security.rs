use std::fs::{File, OpenOptions};
use std::io::{prelude::*, Result};
use std::path::Path;
use openssl::rsa::{Rsa, Padding};
use openssl::pkey::{Public, Private};
use openssl::base64;
use log::info;

#[derive(Clone)]
pub struct SecurityManager {
    pub public_key: Rsa<Public>,
    pub private_key: Rsa<Private>,
}

impl Default for SecurityManager {
    fn default() -> Self {
        let public_key = Path::new("public_key");
        let private_key = Path::new("private_key");
        if !public_key.exists() || !private_key.exists() {
            info!("Creating new RSA keys...");
            SecurityManager::create_keys(public_key, private_key).unwrap();
        }
        let mut public_buf = vec![0u8;2048];
        let mut private_buf = vec![0u8;2048];
        File::open(public_key).unwrap().read(&mut public_buf).unwrap();
        File::open(private_key).unwrap().read(&mut private_buf).unwrap();
        SecurityManager {
            public_key: Rsa::public_key_from_der(&public_buf).unwrap(),
            private_key: Rsa::private_key_from_der(&private_buf).unwrap(),
        }
    }
}

impl SecurityManager {
    pub fn decrypt(&self, text: &str) -> String {
        let mut result: Vec<u8> = vec![0; self.private_key.size() as usize];
        let len: usize = self.private_key
            .private_decrypt(&base64::decode_block(text).unwrap(), &mut result, Padding::PKCS1).unwrap();
        result.truncate(len);
        String::from_utf8(result).unwrap()
    }

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