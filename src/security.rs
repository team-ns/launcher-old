use log::info;
use rand::rngs::OsRng;
use rsa::{PaddingScheme, RSAPrivateKey};
use rsa_export::pem::EncodingScheme;
use rsa_export::{pem, pkcs1};
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, Result};
use std::path::Path;

#[derive(Clone)]
pub struct SecurityManager {
    pub private_key: RSAPrivateKey,
}

impl Default for SecurityManager {
    fn default() -> Self {
        let public_key = Path::new("public_key");
        let private_key = Path::new("private_key");
        if !public_key.exists() || !private_key.exists() {
            info!("Creating new RSA keys...");
            SecurityManager::create_keys(public_key, private_key).unwrap();
        }
        let mut file_content = String::new();
        File::open("private_key")
            .unwrap()
            .read_to_string(&mut file_content);
        let private_key = RSAPrivateKey::try_from(rsa::pem::parse(file_content).unwrap())
            .expect("Failed to parse key!");
        SecurityManager { private_key }
    }
}

impl SecurityManager {
    pub fn decrypt(&self, text: &str) -> String {
        let result = self.private_key.decrypt(
            PaddingScheme::PKCS1v15,
            base64::decode(text).unwrap().as_ref(),
        );
        String::from_utf8(result.unwrap()).unwrap()
    }

    fn create_keys(public_key: &Path, private_key: &Path) -> Result<()> {
        let key = RSAPrivateKey::new(&mut OsRng, 2048).expect("Failed to generate a key!");
        let private = pem::encode(
            EncodingScheme::PKCS1Private,
            pkcs1::private_key(&key).unwrap(),
        );
        let public = pem::encode(
            EncodingScheme::PKCS1Public,
            pkcs1::public_key(&key.to_public_key()).unwrap(),
        );
        SecurityManager::create_key(public_key, public)?;
        SecurityManager::create_key(private_key, private)?;
        Ok(())
    }

    fn create_key(path: &Path, string: String) -> Result<()> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)?
            .write_all(string.as_bytes())
    }
}
