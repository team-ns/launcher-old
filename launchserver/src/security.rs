use anyhow::Result;
use ecies_ed25519::SecretKey;
use log::info;
use rand::rngs::OsRng;
use rand::Rng;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::Path;

pub struct SecurityService {
    pub secret_key: SecretKey,
}

#[teloc::inject]
impl SecurityService {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for SecurityService {
    fn default() -> Self {
        let public_key = Path::new("public_key");
        let secret_key = Path::new("secret_key");
        if !public_key.exists() || !secret_key.exists() {
            info!("Creating new EC KeyPair...");
            Self::create_keys(public_key, secret_key).expect("Failed to create keys!");
        }
        let mut bytes = Vec::new();
        File::open("secret_key")
            .expect("Failed to get secret_key, try restart launch_server!")
            .read_to_end(&mut bytes)
            .expect("Failed to read secret_key, try delete it and restart launch_server!");
        log::info!("Read EC KeyPair");
        Self {
            secret_key: SecretKey::from_bytes(&bytes).expect("Failed to parse key!"),
        }
    }
}

impl SecurityService {
    pub fn decrypt(&self, text: &str) -> Result<String> {
        let text =
            base64::decode(text).map_err(|e| anyhow::anyhow!("Can't decode base64: {:?}!", e))?;
        let pwd = ecies_ed25519::decrypt(&self.secret_key, &text)
            .map_err(|e| anyhow::anyhow!("Invalid encrypted password: {:?}!", e))?;
        String::from_utf8(pwd).map_err(|_| anyhow::anyhow!("Password contains invalid symbols!"))
    }

    fn create_keys(public_key: &Path, secret_key: &Path) -> Result<()> {
        let (secret, public) = ecies_ed25519::generate_keypair(&mut OsRng);
        SecurityService::create_key(public_key, public.to_bytes())?;
        SecurityService::create_key(secret_key, secret.to_bytes())?;
        Ok(())
    }

    fn create_key(path: &Path, bytes: [u8; 32]) -> Result<()> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)?
            .write_all(&bytes)?;
        Ok(())
    }

    pub fn create_access_token() -> String {
        let digest = {
            let mut rng = rand::thread_rng();
            md5::compute(format!(
                "{}{}{}",
                rng.gen_range(1000000000, 2147483647),
                rng.gen_range(1000000000, 2147483647),
                rng.gen_range(0, 9)
            ))
        };
        format!("{:x}", digest)
    }
}
