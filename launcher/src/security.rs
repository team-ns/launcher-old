use ecies_ed25519::PublicKey;

use rand::rngs::OsRng;

pub mod validation;
mod watcher;

#[cfg(feature = "bundle")]
pub fn get_manager() -> SecurityManager {
    SecurityManager {
        public_key: PublicKey::from_bytes(&include_crypt!("public_key").decrypt()).unwrap(),
    }
}

#[cfg(not(feature = "bundle"))]
pub fn get_manager() -> SecurityManager {
    use std::fs;
    SecurityManager {
        public_key: PublicKey::from_bytes(
            &fs::read("public_key").expect("Can't read public key file"),
        )
        .unwrap(),
    }
}

pub struct SecurityManager {
    public_key: PublicKey,
}

impl SecurityManager {
    pub fn encrypt(&self, text: &str) -> String {
        let msg = ecies_ed25519::encrypt(&self.public_key, text.as_bytes(), &mut OsRng);
        base64::encode(msg.unwrap())
    }
}
