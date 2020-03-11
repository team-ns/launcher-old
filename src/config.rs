use std::path::Path;
use std::fs::{File, OpenOptions};
use serde::{Deserialize, Serialize};
use std::clone::Clone;

use crate::config::AuthProvider::{Empty, JSON};
use crate::config::auth::{AuthProvide};
use crate::security::{SecurityManager};

pub(crate) mod auth;
mod texture;

pub fn get_config() -> std::io::Result<Config> {
    let config_path = Path::new("config.json");
    let config_file = OpenOptions::new()
        .write(true)
        .create(true)
        .read(true)
        .open(config_path)?;
    match serde_json::from_reader(&config_file) {
        Ok(config) => Ok(config),
        Err(e) => {
            let config = Config::default();
            serde_json::to_writer_pretty(&config_file, &config)?;
            Ok(config)
        },
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub address: String,
    pub port: u32,
    pub auth: AuthProvider,
    pub texture: TextureProvider,
    #[serde(skip)]
    pub security: SecurityManager,
    pub workers: usize,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextureProvider {
     skin_url: String,
     cape_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum AuthProvider {
    Empty,
    JSON(JsonAuthProvider),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonAuthProvider {
    pub auth_url: String,
    pub entry_url: String,
    pub update_server_id_url: String,
    pub update_access_token_url: String,
}


impl Default for Config {
    fn default() -> Self {
        Config {
            workers: 3,
            address: "127.0.0.1".to_string(),
            port: 8080,
            auth: Empty,
            texture: TextureProvider {
                skin_url: "http://example.com/skin/{}.png".to_string(),
                cape_url: "http://example.com/cape/{}.png".to_string()
            },
            security: SecurityManager::default(),
        }
    }
}
pub struct None;
impl AuthProvider {
    pub fn get_provide<'a> (&'a self) -> Box<dyn AuthProvide> {
        match self.clone() {
            Empty => { Box::new(None {}) }
            JSON(auth) => { Box::new(auth) }
        }
    }
}
