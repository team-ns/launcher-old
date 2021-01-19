use ecies_ed25519::PublicKey;
use rand::rngs::OsRng;

pub mod validation;
mod watcher;

lazy_static_include_bytes! {
    PUBLIC_KEY => "../public_key"
}

pub fn get_manager() -> SecurityManager {
    SecurityManager {
        public_key: PublicKey::from_bytes(*PUBLIC_KEY).unwrap(),
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
