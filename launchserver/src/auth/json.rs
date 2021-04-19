use crate::auth::{AuthProvide, AuthResult, Entry};
use crate::config::auth::JsonAuthConfig;
use crate::security::SecurityService;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName};
use reqwest::{Client, Response};
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
            .map(map_request)
            .unwrap_or_else(|_| Err(anyhow::anyhow!("Can't connect".to_string())))
    }
}

#[derive(Deserialize, Serialize)]
pub struct JsonAuthResult {
    pub uuid: Option<Uuid>,
    pub message: Option<String>,
}

#[async_trait]
impl AuthProvide for JsonAuthProvider {
    async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<AuthResult> {
        let client = &self.client;

        let result: JsonAuthResult = client
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
            let uuid = result.uuid.context("Can't find UUID in auth response")?;
            let access_token = SecurityService::create_access_token();
            self.update_access_token(&uuid, &access_token).await?;
            Ok(AuthResult { uuid, access_token })
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
            .map(map_request)
            .unwrap_or_else(|_| Err(anyhow::anyhow!("Can't connect".to_string())))
    }
}

fn map_request(response: Response) -> Result<()> {
    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Bad request, status code: {}",
            response.status()
        ))
    }
}
