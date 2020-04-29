use crate::client::WebSocketClient;
use config::Config;
use launcher_api::config::Configurable;
use std::error::Error;

mod client;
mod config;
mod game;
mod runtime;
mod security;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    runtime::start().await;
    Ok(())
    /* let config = Config::get_config(
        dirs::config_dir().unwrap()
            .join("nsl")
            .join("config.json")
            .as_path()
    ).unwrap();
    let client = game::Client{name: String::from("test")};
    game::Client::start(&client, &config.game_dir);*/
}
