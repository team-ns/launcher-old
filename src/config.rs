use launcher_api::config::Configurable;
use launcher_api::message::Error;
use serde::{Deserialize, Serialize};
use std::clone::Clone;
use log::error;
use uuid::Uuid;

use crate::config::auth::{AuthProvide, AuthResult, Entry};
use crate::config::AuthProvider::{Empty, JSON};

pub(crate) mod auth;
mod texture;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub address: String,
    pub auth: AuthProvider,
    pub texture: TextureProvider,
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

impl Configurable for Config {}

impl Default for Config {
    fn default() -> Self {
        Config {
            workers: 3,
            address: "127.0.0.1:8080".to_string(),
            auth: Empty,
            texture: TextureProvider {
                skin_url: "http://example.com/skin/{username}.png".to_string(),
                cape_url: "http://example.com/cape/{username}.png".to_string(),
            },
        }
    }
}

pub struct None;

impl AuthProvider {
    pub async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<AuthResult, String> {
        match self {
            Empty => {
                error!("Auth provider not found, check your config!");
                Err("Can't authorize account. Please contact to administration!".to_string())
            },
            JSON(json) => {
                json.auth(login, password, ip).await
                /*let client = reqwest::Client::new();
                let result = client
                    .post(&json.auth_url)
                    .json(&serde_json::json!({
                        "username": login,
                        "password": password,
                        "ip": ip
                    }))
                    .send()
                    .await
                    .map_err(|_e| Error {
                        msg: "Can't connect".to_string(),
                    })?
                    .json()
                    .map_err(|_e| Error {
                        msg: "Can't parse json".to_string(),
                    })
                    .await?;
                Ok(result)*/
            }
        }
    }

    pub async fn get_entry(&self, uuid: &Uuid) -> Result<Entry, Error> {
        match self {
            Empty => Err(Error {
                msg: "Cringe".to_string(),
            }),
            JSON(json) => {
                json.get_entry(uuid).await
                /* let client = reqwest::Client::new();
                Ok(client
                    .post(&json.entry_url)
                    .json(&serde_json::json!({ "uuid": uuid }))
                    .send()
                    .await
                    .map_err(|_e| Error {
                        msg: "Can't connect".to_string(),
                    })?
                    .json()
                    .map_err(|_e| Error {
                        msg: "Can't parse json".to_string(),
                    })
                    .await?)*/
            }
        }
    }
    pub async fn get_entry_from_name(&self, username: &str) -> Result<Entry, Error> {
        match self {
            Empty => Err(Error {
                msg: "Cringe".to_string(),
            }),
            JSON(json) => {
                json.get_entry_from_name(username).await
                /*let client = reqwest::Client::new();
                Ok(client
                    .post(&json.entry_url)
                    .json(&serde_json::json!({ "username": username }))
                    .send()
                    .await
                    .map_err(|_e| Error {
                        msg: "Can't connect".to_string(),
                    })?
                    .json()
                    .map_err(|_e| Error {
                        msg: "Can't parse json".to_string(),
                    })
                    .await?)*/
            }
        }
    }
    pub async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<(), String> {
        match self {
            Empty => {
                error!("Auth provider not found, check your config!");
                Err("Can't authorize account. Please contact to administration!".to_string())
            },
            JSON(json) => {
                json.update_access_token(uuid, token).await
                /*let client = reqwest::Client::new();
                client
                    .post(&json.update_access_token_url)
                    .json(&serde_json::json!({
                        "uuid": uuid,
                        "accessToken": token
                    }))
                    .send()
                    .await
                    .unwrap()
                    .status()
                    .is_success()*/
            }
        }
    }
    pub async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> bool {
        match self {
            JSON(json) => {
                json.update_server_id(uuid, server_id).await
                /* let client = reqwest::Client::new();
                let response = client
                    .post(&json.update_server_id_url)
                    .json(&serde_json::json!({
                    "uuid": uuid,
                    "serverId": server_id
                    }))
                    .send()
                    .await;*/
            }
            Empty => false,
        }
    }
}
