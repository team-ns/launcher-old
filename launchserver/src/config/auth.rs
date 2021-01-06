use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::JsonAuthProvider;
use reqwest::header::HeaderName;
use std::str::FromStr;
use warp::http::HeaderMap;

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
    fn init(&mut self) -> Result<()>;
    async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<AuthResult>;
    async fn get_entry(&self, uuid: &Uuid) -> Result<Entry>;
    async fn get_entry_from_name(&self, username: &str) -> Result<Entry>;
    async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<()>;
    async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> Result<()>;
}

#[async_trait]
impl AuthProvide for JsonAuthProvider {
    fn init(&mut self) -> Result<()> {
        let headers = {
            let mut map = HeaderMap::new();
            map.insert(
                HeaderName::from_str("X-Launcher-API-Key").unwrap(),
                self.api_key.parse().unwrap(),
            );
            map
        };
        let client = Client::builder().default_headers(headers).build()?;
        self.client = Some(client);
        Ok(())
    }

    async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<AuthResult> {
        let client = self.client.as_ref().unwrap();

        let result = client
            .post(&self.auth_url)
            .json(&serde_json::json!({
                "username": login,
                "password": password,
                "ip": ip
            }))
            .send()
            .await?
            .json()
            .await?;
        Ok(result)
    }

    async fn get_entry(&self, uuid: &Uuid) -> Result<Entry> {
        let client = self.client.as_ref().unwrap();

        Ok(client
            .post(&self.entry_url)
            .json(&serde_json::json!({ "uuid": uuid }))
            .send()
            .await?
            .json()
            .await?)
    }

    async fn get_entry_from_name(&self, username: &str) -> Result<Entry> {
        let client = self.client.as_ref().unwrap();

        Ok(client
            .post(&self.entry_url)
            .json(&serde_json::json!({ "username": username }))
            .send()
            .await?
            .json()
            .await?)
    }

    async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<()> {
        let client = self.client.as_ref().unwrap();

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
                    return Err(anyhow::anyhow!("Bad request, status code: {}", v.status()));
                }
            })
            .unwrap_or(Err(anyhow::anyhow!("Can't connect".to_string())))
    }

    async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> Result<()> {
        let client = self.client.as_ref().unwrap();

        client
            .post(&self.update_server_id_url)
            .json(&serde_json::json!({
            "uuid": uuid,
            "serverId": server_id
            }))
            .send()
            .await?;
        Ok(())
    }
}
