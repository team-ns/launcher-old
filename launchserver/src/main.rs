use launcher_api::config::Configurable;
use launcher_api::profile::ProfileData;
use log::info;
use server::profile;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::AuthProvider;
use crate::config::Config;
use crate::security::SecurityManager;

mod auth;
mod bundle;
mod commands;
mod config;
mod logger;
mod security;
mod server;

pub struct LaunchServer {
    pub config: Config,
    pub security: SecurityManager,
    pub profiles_data: HashMap<String, ProfileData>,
    pub auth_provider: AuthProvider,
}

impl LaunchServer {
    async fn new() -> Self {
        logger::configure();
        bundle::unpack_launcher();
        info!("Read config file...");
        let config = Config::get_config(Path::new("config.json")).expect("Can't read config file!");
        info!("Launch server starting...");
        let mut security = SecurityManager::default();
        let profiles_data = profile::get_profiles_data();
        security.rehash(
            profiles_data.values().map(|data| &data.profile),
            &[],
            config.file_server.clone(),
        );
        let auth_provider = config
            .auth
            .get_provider()
            .await
            .expect("Can't initialize auth provider");

        LaunchServer {
            config,
            security,
            profiles_data,
            auth_provider,
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let data = Arc::new(RwLock::new(LaunchServer::new().await));
    tokio::join!(commands::start(data.clone()), server::start(data.clone()));
    Ok(())
}
