use launcher_api::config::Configurable;
use config::Config;
use crate::client::WebSocketClient;
use std::thread;
use std::time::Duration;

mod game;
mod security;
mod config;
mod client;
mod runtime;

fn main() {
    env_logger::init();
    let mut socket = WebSocketClient::new("ws://127.0.0.1:8080/api/");
    thread::sleep(Duration::from_secs(5));
    socket.auth("Test", "test");
   /* let config = Config::get_config(
        dirs::config_dir().unwrap()
            .join("nsl")
            .join("config.json")
            .as_path()
    ).unwrap();
    let client = game::Client{name: String::from("test")};
    game::Client::start(&client, &config.game_dir);*/
}

