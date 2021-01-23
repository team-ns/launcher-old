use crate::config::auth::{AuthProvide, Entry};
use anyhow::Result;
use async_trait::async_trait;

use serde::{Deserialize, Serialize};

use sqlx::{AnyPool, Row};

use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SqlAuthProvider {
    pub connection_url: String,
    pub fetch_entry_username_query: String,
    pub fetch_entry_uuid_query: String,
    pub auth_query: String,
    pub auth_message: String,
    pub update_server_id_query: String,
    pub update_access_token_query: String,
    #[serde(skip)]
    pub client: Option<AnyPool>,
}
#[async_trait]
impl AuthProvide for SqlAuthProvider {
    async fn init(&mut self) -> Result<()> {
        let pool = AnyPool::connect(&self.connection_url).await?;
        self.client = Some(pool);
        Ok(())
    }

    async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<Uuid> {
        let client = self.client.as_ref().unwrap();
        let row = sqlx::query(&self.auth_query)
            .bind(login)
            .bind(password)
            .bind(ip)
            .fetch_optional(client)
            .await?;

        match row {
            Some(_) => Ok(self.get_entry_from_name(login).await?.uuid),
            None => Err(anyhow::anyhow!("{}", self.auth_message)),
        }
    }

    async fn get_entry(&self, uuid: &Uuid) -> Result<Entry> {
        let client = self.client.as_ref().unwrap();
        let row = sqlx::query(&self.fetch_entry_uuid_query)
            .bind(&uuid)
            .fetch_one(client)
            .await?;

        Ok(Entry {
            access_token: row.get("access_token"),
            server_id: row.get("server_id"),
            uuid: row.get("uuid"),
            username: row.get("username"),
        })
    }

    async fn get_entry_from_name(&self, username: &str) -> Result<Entry> {
        let client = self.client.as_ref().unwrap();

        let row = sqlx::query(&self.fetch_entry_username_query)
            .bind(username)
            .fetch_one(client)
            .await?;

        Ok(Entry {
            access_token: row.get("access_token"),
            server_id: row.get("server_id"),
            uuid: row.get("uuid"),
            username: row.get("username"),
        })
    }

    async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<()> {
        let client = self.client.as_ref().unwrap();
        sqlx::query(&self.update_access_token_query)
            .bind(token)
            .bind(&uuid)
            .execute(client)
            .await?;
        Ok(())
    }

    async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> Result<()> {
        let client = self.client.as_ref().unwrap();
        sqlx::query(&self.update_access_token_query)
            .bind(server_id)
            .bind(&uuid)
            .execute(client)
            .await?;
        Ok(())
    }
}
