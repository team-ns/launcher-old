use std::collections::HashMap;
use std::time::{Instant, Duration};

use serde_json::Value;
use strfmt::Format;
use uuid::Uuid;

use crate::config::auth::Entry;
use crate::config::TextureProvider;

impl TextureProvider {
    pub fn get_skin_url(&self, entry: &Entry) -> Option<String> {
        let mut vars = HashMap::new();
        vars.insert("username".to_string(), entry.username.to_string());
        vars.insert("job".to_string(), entry.uuid.to_string());
        Some((&self.skin_url.format(&vars).unwrap()).to_string())
    }

    pub fn get_cape_url(&self, entry: &Entry) -> Option<String> {
        let mut vars = HashMap::new();
        vars.insert("username".to_string(), entry.username.to_string());
        vars.insert("job".to_string(), entry.uuid.to_string());
        Some((&self.cape_url.format(&vars).unwrap()).to_string())
    }

    pub fn get_textures_property(&self, entry: &Entry) -> Value {
        let timestamp = 0;
        serde_json::json!({
            "timestamp": timestamp,
            "profileId": entry.uuid.to_simple().encode_lower(&mut Uuid::encode_buffer()),
            "profileName": entry.username,
            "textures": {
                "SKIN": {
                    "url": self.get_skin_url(entry)
                },
                "CAPE": {
                    "url": self.get_cape_url(entry)
                }
            }
        })
    }
}


