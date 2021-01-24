use crate::auth::{AuthProvide, Entry};
use anyhow::{Context, Result};
use async_trait::async_trait;
use multi_map::MultiMap;

use tokio::sync::Mutex;
use uuid::Uuid;

pub struct AcceptAuthProvider {
    pub cache: Mutex<MultiMap<Uuid, String, Entry>>,
}

impl Default for AcceptAuthProvider {
    fn default() -> Self {
        AcceptAuthProvider {
            cache: Mutex::new(MultiMap::new()),
        }
    }
}

#[async_trait]
impl AuthProvide for AcceptAuthProvider {
    async fn auth(&self, login: &str, _password: &str, _ip: &str) -> Result<Uuid> {
        let mut cache = self.cache.lock().await;
        let uuid = Uuid::new_v4();
        let entry = Entry {
            access_token: None,
            server_id: None,
            uuid,
            username: login.to_string(),
        };
        cache.insert(uuid, login.to_string(), entry);
        Ok(uuid)
    }

    async fn get_entry(&self, uuid: &Uuid) -> Result<Entry> {
        let cache = self.cache.lock().await;
        let entry = cache.get(uuid).context("Entry not found")?;
        Ok(entry.clone())
    }

    async fn get_entry_from_name(&self, username: &str) -> Result<Entry> {
        let cache = self.cache.lock().await;
        let entry = cache.get_alt(username).context("Entry not found")?;
        Ok(entry.clone())
    }

    async fn update_access_token(&self, uuid: &Uuid, token: &str) -> Result<()> {
        let mut cache = self.cache.lock().await;
        let mut entry = cache.get_mut(uuid).context("Entry not found")?;
        entry.access_token = Some(token.to_string());
        Ok(())
    }

    async fn update_server_id(&self, uuid: &Uuid, server_id: &str) -> Result<()> {
        let mut cache = self.cache.lock().await;
        let mut entry = cache.get_mut(uuid).context("Entry not found")?;
        entry.server_id = Some(server_id.to_string());
        Ok(())
    }
}
