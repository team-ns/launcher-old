use env_logger::Env;
use log::info;
use std::path::Path;
use launcher_api::config::Configurable;

use crate::config::Config;

mod server;
mod config;
mod security;
mod commands;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::from_env(Env::default().default_filter_or("launch_server,actix_web=debug")).init();
    info!("Read config file...");
    let config = Config::get_config(Path::new("config.json"))?;
    commands::start(config.clone());
    info!("Launch server starting...");
    server::start(config).await
}

