use openssl::base64;
use openssl::pkey::Public;
use openssl::rsa::{Padding, Rsa};

pub fn get_manager() -> SecurityManager {
    SecurityManager {
        public_key: Rsa::public_key_from_der(include_bytes!("../public_key")).unwrap(),
    }
}

pub struct SecurityManager {
    public_key: Rsa<Public>,
}

impl SecurityManager {
    pub fn encrypt(&self, text: &str) -> String {
        let mut result: Vec<u8> = vec![0; self.public_key.size() as usize];
        let len: usize = self
            .public_key
            .public_encrypt(text.as_bytes(), &mut result, Padding::PKCS1)
            .unwrap();
        result.truncate(len);
        base64::encode_block(&result)
    }
}
