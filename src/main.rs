use launcher_api::config::Configurable;
use config::Config;
use std::path::Path;

mod game;
mod security;
mod config;
mod client;

fn main() {
    let config = Config::get_config(
        dirs::config_dir().unwrap()
            .join("nsl")
            .join("config.json")
            .as_path()
    ).unwrap();
    let client = game::Client{name: String::from("test")};
    game::Client::start(&client, &config.game_dir);
}

