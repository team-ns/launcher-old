use launcher_api::config::Configurable;
use log::info;
use std::path::Path;

use crate::config::Config;
use crate::security::SecurityManager;
use launcher_api::profile::{Profile, ProfileInfo};
use server::profile;
use std::sync::Arc;
use tokio::sync::RwLock;

mod commands;
mod config;
mod security;
mod server;

pub struct LaunchServer {
    pub config: Config,
    pub security: SecurityManager,
    pub profiles: Vec<Profile>,
    pub profiles_info: Vec<ProfileInfo>,
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
        let (profiles, profiles_info) = profile::get_profiles();
        LaunchServer {
            config,
            security: SecurityManager::default(),
            profiles,
            profiles_info,
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let data = Arc::new(RwLock::new(LaunchServer::new().await));
    commands::start(data.clone()).await;
    server::start(data).await
}
