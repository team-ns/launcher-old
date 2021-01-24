use anyhow::Result;
use async_trait::async_trait;

use sqlx::{AnyPool, Row};

use crate::auth::{AuthProvide, Entry};
use crate::config::auth::SqlAuthConfig;
use uuid::Uuid;

pub struct SqlAuthProvider {
    pub config: SqlAuthConfig,
    pub pool: AnyPool,
}

impl SqlAuthProvider {
    pub async fn new(config: SqlAuthConfig) -> Result<Self> {
        let pool = AnyPool::connect(&config.connection_url).await?;
        Ok(SqlAuthProvider { config, pool })
    }
}

#[async_trait]
impl AuthProvide for SqlAuthProvider {
    async fn auth(&self, login: &str, password: &str, ip: &str) -> Result<Uuid> {
        let pool = &self.pool;
        let row = sqlx::query(&self.config.auth_query)
            .bind(login)
            .bind(password)
            .bind(ip)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(_) => Ok(self.get_entry_from_name(login).await?.uuid),
            None => Err(anyhow::anyhow!("{}", self.config.auth_message)),
        }
    }

    async fn get_entry(&self, uuid: &Uuid) -> Result<Entry> {
        let pool = &self.pool;

        let row = sqlx::query(&self.config.fetch_entry_uuid_query)
            .bind(&uuid)
            .fetch_one(pool)
            .await?;

        Ok(Entry {
            access_token: row.get("access_token"),
            server_id: row.get("server_id"),
            uuid: row.get("uuid"),
            username: row.get("username"),
        })
    }

    async fn get_entry_from_name(&self, username: &str) -> Result<Entry> {
        let pool = &self.pool;

        let row = sqlx::query(&self.config.fetch_entry_username_query)
            .bind(username)
            .fetch_one(pool)
            .await?;

        Ok(Entry {
            access_token: row.get("access_token"),
            server_id: row.get("server_id"),
            uuid: row.get("uuid"),
            username: row.get("username"),
        })
    }

    async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<()> {
        let pool = &self.pool;

        sqlx::query(&self.config.update_access_token_query)
            .bind(token)
            .bind(&uuid)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> Result<()> {
        let pool = &self.pool;

        sqlx::query(&self.config.update_access_token_query)
            .bind(server_id)
            .bind(&uuid)
            .execute(pool)
            .await?;
        Ok(())
    }
}
