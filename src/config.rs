use launcher_api::config::Configurable;
use serde::{Serialize, Deserialize};
use path_slash::PathExt;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub game_dir: String,
    pub save_data: bool,
    pub saved_password: String,
    pub last_name: String,
    pub url: String,
}

impl Configurable for Config { }

impl Default for Config {
    fn default() -> Self {
        let config_json = include_str!("../config.json")
                        .replace("%homeDir%", &dirs::home_dir().unwrap()
                        .to_slash_lossy());
        serde_json::from_str(&config_json).unwrap()
    }
}