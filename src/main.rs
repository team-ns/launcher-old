use launcher_api::config::Configurable;
use config::Config;
use crate::client::WebSocketClient;

mod game;
mod security;
mod config;
mod client;
mod runtime;

fn main() {
    env_logger::init();
    let mut socket = WebSocketClient::new("ws://127.0.0.1:8080/api/");
    runtime::start(socket);
   /* let config = Config::get_config(
        dirs::config_dir().unwrap()
            .join("nsl")
            .join("config.json")
            .as_path()
    ).unwrap();
    let client = game::Client{name: String::from("test")};
    game::Client::start(&client, &config.game_dir);*/
}

