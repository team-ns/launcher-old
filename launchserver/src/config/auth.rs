use anyhow::Result;
use async_trait::async_trait;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) mod json;
pub(crate) mod sql;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub access_token: Option<String>,
    pub server_id: Option<String>,
    pub uuid: Uuid,
    pub username: String,
}

#[async_trait]
pub trait AuthProvide {
    async fn init(&mut self) -> Result<()>;
    async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<Uuid>;
    async fn get_entry(&self, uuid: &Uuid) -> Result<Entry>;
    async fn get_entry_from_name(&self, username: &str) -> Result<Entry>;
    async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<()>;
    async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> Result<()>;
}
