use anyhow::Result;
use launcher_api::config::Configurable;
use once_cell::sync::{Lazy, OnceCell};
use path_slash::PathExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;
use std::{fs, path};
use tokio::sync::Mutex;

pub static CONFIG: Lazy<Config> = Lazy::new(Config::default);

pub static SETTINGS: OnceCell<Arc<Mutex<Settings>>> = OnceCell::new();

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub game_dir: String,
    pub websocket: String,
    pub ram: u64,
    pub project_name: String,
}

impl Configurable for Config {}

impl Default for Config {
    fn default() -> Self {
        let config_json = obfstr::obfstr!(include_str!("../config.json"))
            .replace("%homeDir%", &dirs::home_dir().unwrap().to_slash_lossy());
        serde_json::from_str(&config_json).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub game_dir: String,
    pub save_data: bool,
    pub ram: u64,
    pub saved_password: Option<String>,
    pub last_name: Option<String>,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let path = path::Path::new(&CONFIG.game_dir).join("settings.bin");
        let settings = bincode::deserialize::<Self>(&fs::read(path)?)?;
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let body = bincode::serialize(self)?;
        let path = path::Path::new(&CONFIG.game_dir).join("settings.bin");
        let mut file = fs::File::create(path)?;
        file.write_all(&body)?;
        Ok(())
    }

    pub fn update(&mut self, settings: &Self) -> Result<()> {
        self.ram = settings.ram;
        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            game_dir: CONFIG.game_dir.clone(),
            save_data: false,
            ram: CONFIG.ram,
            saved_password: None,
            last_name: None,
        }
    }
}
