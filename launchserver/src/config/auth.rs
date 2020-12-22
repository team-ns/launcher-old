use async_trait::async_trait;
use futures::TryFutureExt;
use launcher_api::message::Error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{JsonAuthProvider, None};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub access_token: Option<String>,
    pub server_id: Option<String>,
    pub uuid: Uuid,
    pub username: String,
}

#[derive(Deserialize, Serialize)]
pub struct AuthResult {
    pub uuid: Option<Uuid>,
    pub message: Option<String>,
}

#[async_trait]
pub trait AuthProvide {
    async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<AuthResult, String>;
    async fn get_entry(&self, uuid: &Uuid) -> Result<Entry, Error>;
    async fn get_entry_from_name(&self, username: &str) -> Result<Entry, Error>;
    async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<(), String>;
    async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> bool;
}

#[async_trait]
impl AuthProvide for JsonAuthProvider {
    async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<AuthResult, String> {
        let client = Client::default();

        let result = client
            .post(&self.auth_url)
            .json(&serde_json::json!({
                "username": login,
                "password": password,
                "ip": ip
            }))
            .send()
            .await
            .map_err(|_| "Can't connect".to_string())?
            .json()
            .map_err(|_| "Can't parse json".to_string())
            .await?;
        Ok(result)
    }

    async fn get_entry(&self, uuid: &Uuid) -> Result<Entry, Error> {
        let client = Client::default();
        Ok(client
            .post(&self.entry_url)
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
            .await?)
    }

    async fn get_entry_from_name(&self, username: &str) -> Result<Entry, Error> {
        let client = Client::default();
        Ok(client
            .post(&self.entry_url)
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
            .await?)
    }

    async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<(), String> {
        let client = Client::default();
        client
            .post(&self.update_access_token_url)
            .json(&serde_json::json!({
                "uuid": uuid,
                "accessToken": token
            }))
            .send()
            .await
            .map(|v| {
                if v.status().is_success() {
                    Ok(())
                } else {
                    return Err(format!("Bad request, status code: {}", v.status()));
                }
            })
            .unwrap_or(Err("Can't connect".to_string()))
    }

    async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> bool {
        let client = Client::default();
        client
            .post(&self.update_server_id_url)
            .json(&serde_json::json!({
            "uuid": uuid,
            "serverId": server_id
            }))
            .send()
            .await
            .map(|v| v.status().is_success())
            .unwrap_or(false)
    }
}

#[async_trait]
impl AuthProvide for None {
    async fn auth(&self, _login: &str, _password: &str, _ip: &str) -> Result<AuthResult, String> {
        unimplemented!()
    }

    async fn get_entry(&self, _uuid: &Uuid) -> Result<Entry, Error> {
        unimplemented!()
    }

    async fn get_entry_from_name(&self, _username: &str) -> Result<Entry, Error> {
        unimplemented!()
    }

    async fn update_access_token(&self, _uuid: &Uuid, _token: &str) -> Result<(), String> {
        unimplemented!()
    }

    async fn update_server_id(&self, _uuid: &Uuid, _server_id: &str) -> bool {
        unimplemented!()
    }
}
