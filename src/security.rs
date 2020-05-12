use rand::rngs::OsRng;
use rsa::{PaddingScheme, PublicKey, RSAPublicKey};
use std::convert::TryFrom;

pub fn get_manager() -> SecurityManager {
    let bytes = include_bytes!("../public_key");
    SecurityManager {
        public_key: RSAPublicKey::try_from(
            rsa::pem::parse(String::from_utf8_lossy(bytes).to_string()).unwrap(),
        )
        .unwrap(),
    }
}

pub struct SecurityManager {
    public_key: RSAPublicKey,
}

impl SecurityManager {
    pub fn encrypt(&self, text: &str) -> String {
        let msg = self
            .public_key
            .encrypt(&mut OsRng, PaddingScheme::PKCS1v15, text.as_ref());
        base64::encode(msg.unwrap())
    }
}
