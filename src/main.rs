use launcher_api::config::Configurable;
use launcher_api::profile::{Profile, ProfileInfo};
use launcher_api::message::{ClientMessage, ProfileResourcesMessage};
use launcher_api::validation::OsType;
use server::profile;
use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;
use tokio::sync::RwLock;
use log::info;

use crate::config::Config;
use crate::security::SecurityManager;

mod commands;
mod config;
mod security;
mod server;

pub struct LaunchServer {
    pub config: Config,
    pub security: SecurityManager,
    pub profiles: HashMap<String, Profile>,
    pub profiles_info: Vec<ProfileInfo>,
}

impl LaunchServer {
    async fn new() -> Self {
        env_logger::builder()
            .filter_module("rustyline", log::LevelFilter::Info)
            .filter_level(log::LevelFilter::Debug)
            .init();
        info!("Read config file...");
        let config = Config::get_config(Path::new("config.json")).expect("Can't read config file!");
        info!("Launch server starting...");
        let (profiles, profiles_info) = profile::get_profiles();
        let mut security = SecurityManager::default();
        security.rehash(profiles.values(), &[]);

        LaunchServer {
            config,
            security,
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