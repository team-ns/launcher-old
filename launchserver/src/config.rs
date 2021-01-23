use crate::config::auth::json::JsonAuthProvider;
use crate::config::auth::sql::SqlAuthProvider;
use crate::config::auth::{AuthProvide, Entry};
use crate::config::AuthProvider::Empty;
use anyhow::Result;
use launcher_api::config::Configurable;
use log::error;
use serde::{Deserialize, Serialize};

use std::clone::Clone;
use uuid::Uuid;

pub(crate) mod auth;
mod texture;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub bind_address: String,
    pub auth: AuthProvider,
    pub texture: TextureProvider,
    pub file_server: String,
    pub websocket_url: String,
    pub project_name: String,
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
    SQL(SqlAuthProvider),
}

impl Configurable for Config {}

impl Default for Config {
    fn default() -> Self {
        Config {
            file_server: "http://127.0.0.1:8080/files".to_string(),
            bind_address: "127.0.0.1:8080".to_string(),
            auth: Empty,
            texture: TextureProvider {
                skin_url: "http://example.com/skin/{username}.png".to_string(),
                cape_url: "http://example.com/cape/{username}.png".to_string(),
            },
            websocket_url: "ws://127.0.0.1:8080".to_string(),
            project_name: "NSL".to_string(),
        }
    }
}

impl Config {
    pub async fn init(&mut self) -> Result<()> {
        self.auth.init().await?;
        Ok(())
    }
}

impl AuthProvider {
    pub async fn init(&mut self) -> Result<()> {
        match self {
            Empty => {
                error!("Auth provider not found, check your config!");
                Err(anyhow::anyhow!(
                    "Can't initialize launchserver. Auth provider not configured!".to_string()
                ))
            }
            AuthProvider::JSON(json) => json.init().await,
            AuthProvider::SQL(sql) => sql.init().await,
        }
    }

    pub async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<Uuid> {
        match self {
            Empty => {
                error!("Auth provider not found, check your config!");
                Err(anyhow::anyhow!(
                    "Can't authorize account. Please contact to administration!".to_string()
                ))
            }
            AuthProvider::JSON(json) => json.auth(login, password, ip).await,
            AuthProvider::SQL(sql) => sql.auth(login, password, ip).await,
        }
    }

    pub async fn get_entry(&self, uuid: &Uuid) -> Result<Entry> {
        match self {
            Empty => {
                error!("Auth provider not found, check your config!");
                Err(anyhow::anyhow!(
                    "Can't get account entry. Please contact to administration!".to_string()
                ))
            }
            AuthProvider::JSON(json) => json.get_entry(uuid).await,
            AuthProvider::SQL(sql) => sql.get_entry(uuid).await,
        }
    }
    pub async fn get_entry_from_name(&self, username: &str) -> Result<Entry> {
        match self {
            Empty => {
                error!("Auth provider not found, check your config!");
                Err(anyhow::anyhow!(
                    "Can't get account entry. Please contact to administration!".to_string()
                ))
            }
            AuthProvider::JSON(json) => json.get_entry_from_name(username).await,
            AuthProvider::SQL(sql) => sql.get_entry_from_name(username).await,
        }
    }
    pub async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<()> {
        match self {
            Empty => {
                error!("Auth provider not found, check your config!");
                Err(anyhow::anyhow!(
                    "Can't authorize account. Please contact to administration!".to_string()
                ))
            }
            AuthProvider::JSON(json) => json.update_access_token(uuid, token).await,
            AuthProvider::SQL(sql) => sql.update_access_token(uuid, token).await,
        }
    }
    pub async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> Result<()> {
        match self {
            Empty => {
                error!("Auth provider not found, check your config!");
                Err(anyhow::anyhow!(
                    "Can't authorize account. Please contact to administration!".to_string()
                ))
            }
            AuthProvider::JSON(json) => json.update_server_id(uuid, server_id).await,
            AuthProvider::SQL(sql) => sql.update_server_id(uuid, server_id).await,
        }
    }
}
