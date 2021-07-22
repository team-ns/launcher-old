use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::config::{Settings, SETTINGS};
use crate::runtime::arg::InvokeResolver;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
    Set { key: String, value: Value },
    Get { key: String },
    Remove { key: String },
}

impl Cmd {
    pub async fn run(self, resolver: InvokeResolver) {
        let settings = Arc::clone(SETTINGS.get().expect("Client not found"));
        match self {
            Cmd::Set { key, value } => {
                resolver.resolve_result(set_in_storage(key, value, settings).await)
            }
            Cmd::Get { key } => resolver.resolve_result(get_from_storage(key, settings).await),
            Cmd::Remove { key } => {
                resolver.resolve_result(remove_from_storage(key, settings).await)
            }
        };
    }
}

async fn set_in_storage(key: String, value: Value, settings: Arc<Mutex<Settings>>) -> Result<()> {
    let mut settings = settings.lock().await;
    settings
        .properties
        .insert(key, serde_json::to_string(&value)?);
    settings.save()?;
    Ok(())
}

async fn get_from_storage(key: String, settings: Arc<Mutex<Settings>>) -> Result<Value> {
    let settings = settings.lock().await;
    let value = serde_json::from_str::<Value>(
        settings
            .properties
            .get(&key)
            .context("Can't find property")?,
    )?;
    Ok(value)
}

async fn remove_from_storage(key: String, settings: Arc<Mutex<Settings>>) -> Result<Value> {
    let mut settings = settings.lock().await;
    let value = serde_json::from_str::<Value>(
        &settings
            .properties
            .remove(&key)
            .context("Can't find property")?,
    )?;
    Ok(value)
}
