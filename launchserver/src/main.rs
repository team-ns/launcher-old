use launcher_api::config::Configurable;
use launcher_api::profile::{Profile, ProfileInfo};
use log::info;
use server::profile;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::security::SecurityManager;

mod bundle;
mod commands;
mod config;
mod logger;
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
        logger::configure();
        bundle::unpack_launcher();
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
    tokio::join!(commands::start(data.clone()), server::start(data.clone()));
    Ok(())
}
