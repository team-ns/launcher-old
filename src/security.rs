use ecies_ed25519::PublicKey;
use rand::rngs::OsRng;

pub fn get_manager() -> SecurityManager {
    SecurityManager {
        public_key: PublicKey::from_bytes(include_bytes!("../public_key")).unwrap(),
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
