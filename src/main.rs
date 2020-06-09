use launcher_api::config::Configurable;
use log::info;
use std::path::Path;

use crate::config::Config;
use crate::security::SecurityManager;
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
        env_logger::builder()
            .filter_module("rustyline", log::LevelFilter::Info)
            .filter_level(log::LevelFilter::Debug)
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
    commands::start(data.clone()).await;
    server::start(data).await
}
