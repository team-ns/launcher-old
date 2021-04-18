use crate::auth::{AuthProvide, Entry};
use crate::config::auth::JsonAuthConfig;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

pub struct JsonAuthProvider {
    pub config: JsonAuthConfig,
    pub client: Client,
}

impl JsonAuthProvider {
    pub fn new(config: JsonAuthConfig) -> Result<Self> {
        let headers = {
            let mut map = HeaderMap::new();
            map.insert(
                HeaderName::from_str("X-Launcher-API-Key").unwrap(),
                config.api_key.parse().unwrap(),
            );
            map
        };
        let client = Client::builder().default_headers(headers).build()?;

        Ok(JsonAuthProvider { config, client })
    }
}

#[derive(Deserialize, Serialize)]
pub struct AuthResult {
    pub uuid: Option<Uuid>,
    pub message: Option<String>,
}

#[async_trait]
impl AuthProvide for JsonAuthProvider {
    async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<Uuid> {
        let client = &self.client;

        let result: AuthResult = client
            .post(&self.config.auth_url)
            .json(&serde_json::json!({
                "username": login,
                "password": password,
                "ip": ip
            }))
            .send()
            .await?
            .json()
            .await?;
        if result.message.is_none() {
            Ok(result.uuid.unwrap())
        } else {
            Err(anyhow::anyhow!("{}", result.message.unwrap()))
        }
    }

    async fn get_entry(&self, uuid: &Uuid) -> Result<Entry> {
        let client = &self.client;

        Ok(client
            .post(&self.config.entry_url)
            .json(&serde_json::json!({ "uuid": uuid }))
            .send()
            .await?
            .json()
            .await?)
    }

    async fn get_entry_from_name(&self, username: &str) -> Result<Entry> {
        let client = &self.client;

        Ok(client
            .post(&self.config.entry_url)
            .json(&serde_json::json!({ "username": username }))
            .send()
            .await?
            .json()
            .await?)
    }

    async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<()> {
        let client = &self.client;

        client
            .post(&self.config.update_access_token_url)
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
                    Err(anyhow::anyhow!("Bad request, status code: {}", v.status()))
                }
            })
            .unwrap_or_else(|_| Err(anyhow::anyhow!("Can't connect".to_string())))
    }

    async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> Result<()> {
        let client = &self.client;

        client
            .post(&self.config.update_server_id_url)
            .json(&serde_json::json!({
                "uuid": uuid,
                "serverId": server_id
            }))
            .send()
            .await
            .map(|v| {
                if v.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Bad request, status code: {}", v.status()))
                }
            })
            .unwrap_or_else(|_| Err(anyhow::anyhow!("Can't connect".to_string())))
    }
}
