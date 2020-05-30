use env_logger::Env;
use launcher_api::config::Configurable;
use log::info;
use std::path::Path;

use crate::config::Config;
use crate::security::SecurityManager;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::RwLock;

mod commands;
mod config;
mod security;
mod server;

pub struct LaunchServer {
    pub config: Config,
    pub security: SecurityManager,
}

impl LaunchServer {
    async fn new() -> Self {
        env_logger::from_env(Env::default().default_filter_or("launch_server,actix_web=debug"))
            .init();
        info!("Read config file...");
        let config = Config::get_config(Path::new("config.json")).unwrap();
        info!("Launch server starting...");
        LaunchServer {
            config,
            security: SecurityManager::default(),
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let data = Arc::new(RwLock::new(LaunchServer::new().await));
    info!("Start command module...");
    commands::start(data.clone()).await;
    info!("Start websocket and http server...");
    server::start(data).await
}
