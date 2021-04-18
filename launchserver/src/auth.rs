use anyhow::Result;
use async_trait::async_trait;

use crate::auth::accept::AcceptAuthProvider;
use crate::auth::json::JsonAuthProvider;
use crate::auth::sql::SqlAuthProvider;
use crate::config::Config;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) mod accept;
pub(crate) mod json;
pub(crate) mod sql;

pub enum AuthProvider {
    Json(JsonAuthProvider),
    Sql(SqlAuthProvider),
    Accept(AcceptAuthProvider),
}

#[teloc::inject]
impl AuthProvider {
    pub fn new(config: &Config) -> Self {
        log::info!("Create auth provider");
        config
            .auth
            .get_provider()
            .expect("Can't initialize auth provider")
    }
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub access_token: Option<String>,
    pub server_id: Option<String>,
    pub uuid: Uuid,
    pub username: String,
}

#[async_trait]
pub trait AuthProvide {
    async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<Uuid>;
    async fn get_entry(&self, uuid: &Uuid) -> Result<Entry>;
    async fn get_entry_from_name(&self, username: &str) -> Result<Entry>;
    async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<()>;
    async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> Result<()>;
}

impl AuthProvider {
    pub async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<Uuid> {
        match self {
            AuthProvider::Accept(accept) => accept.auth(login, password, ip).await,
            AuthProvider::Json(json) => json.auth(login, password, ip).await,
            AuthProvider::Sql(sql) => sql.auth(login, password, ip).await,
        }
    }

    pub async fn get_entry(&self, uuid: &Uuid) -> Result<Entry> {
        match self {
            AuthProvider::Accept(accept) => accept.get_entry(uuid).await,
            AuthProvider::Json(json) => json.get_entry(uuid).await,
            AuthProvider::Sql(sql) => sql.get_entry(uuid).await,
        }
    }
    pub async fn get_entry_from_name(&self, username: &str) -> Result<Entry> {
        match self {
            AuthProvider::Accept(accept) => accept.get_entry_from_name(username).await,
            AuthProvider::Json(json) => json.get_entry_from_name(username).await,
            AuthProvider::Sql(sql) => sql.get_entry_from_name(username).await,
        }
    }
    pub async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<()> {
        match self {
            AuthProvider::Accept(accept) => accept.update_access_token(uuid, token).await,
            AuthProvider::Json(json) => json.update_access_token(uuid, token).await,
            AuthProvider::Sql(sql) => sql.update_access_token(uuid, token).await,
        }
    }
    pub async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> Result<()> {
        match self {
            AuthProvider::Accept(accept) => accept.update_server_id(uuid, server_id).await,
            AuthProvider::Json(json) => json.update_server_id(uuid, server_id).await,
            AuthProvider::Sql(sql) => sql.update_server_id(uuid, server_id).await,
        }
    }
}
