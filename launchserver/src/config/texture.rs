use serde_json::Value;
use std::collections::HashMap;
use strfmt::Format;
use uuid::Uuid;

use crate::auth::Entry;
use crate::config::TextureProvider;

impl TextureProvider {
    fn get_url(&self, url: &str, entry: &Entry) -> String {
        let mut vars = HashMap::new();
        vars.insert("username".to_string(), entry.username.to_string());
        vars.insert("uuid".to_string(), entry.uuid.to_string());
        (url.format(&vars)).unwrap_or_else(|_| "".to_string())
    }

    pub fn get_textures_property(&self, entry: &Entry) -> Value {
        let timestamp = 0;
        serde_json::json!({
            "timestamp": timestamp,
            "profileId": entry.uuid.to_simple().encode_lower(&mut Uuid::encode_buffer()),
            "profileName": entry.username,
            "textures": {
                "SKIN": {
                    "url": self.get_url(&self.skin_url, entry)
                },
                "CAPE": {
                    "url": self.get_url(&self.cape_url, entry)
                }
            }
        })
    }
}
