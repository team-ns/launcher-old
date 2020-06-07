use crate::server::profile::HashedProfile;
use ecies_ed25519::SecretKey;
use log::info;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, Result};
use std::path::Path;

pub struct SecurityManager {
    pub secret_key: SecretKey,
    pub profiles: HashMap<String, HashedProfile>,
}

impl Default for SecurityManager {
    fn default() -> Self {
        let public_key = Path::new("public_key");
        let secret_key = Path::new("secret_key");
        if !public_key.exists() || !secret_key.exists() {
            info!("Creating new EC KeyPair...");
            SecurityManager::create_keys(public_key, secret_key).unwrap();
        }
        let mut bytes = Vec::new();
        File::open("secret_key")
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        SecurityManager {
            secret_key: SecretKey::from_bytes(&bytes).expect("Failed to parse key!"),
            profiles: HashMap::new(),
        }
    }
}

impl SecurityManager {
    pub fn decrypt(&self, text: &str) -> String {
        let result =
            ecies_ed25519::decrypt(&self.secret_key, base64::decode(text).unwrap().as_ref());
        String::from_utf8(result.unwrap()).unwrap()
    }

    fn create_keys(public_key: &Path, secret_key: &Path) -> Result<()> {
        let (secret, public) = ecies_ed25519::generate_keypair(&mut OsRng);
        SecurityManager::create_key(public_key, public.to_bytes())?;
        SecurityManager::create_key(secret_key, secret.to_bytes())?;
        Ok(())
    }

    fn create_key(path: &Path, bytes: [u8; 32]) -> Result<()> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)?
            .write_all(&bytes)
    }
}
