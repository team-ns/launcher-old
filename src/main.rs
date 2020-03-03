mod server;
mod config;

use log::info;
use env_logger::Env;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::from_env(Env::default().default_filter_or("launch_server,actix_web=debug")).init();
    info!("Read config file...");
    let config = config::get_config()?;
    info!("Launch server starting...");
    server::start(config).await
}

