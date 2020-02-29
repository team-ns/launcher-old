use std::path::Path;
use std::fs::File;
use serde::{Deserialize, Serialize};

pub fn get_config() ->  std::io::Result<Config> {
    let config_path = Path::new("config.json");
    let config_file = {
        if config_path.exists() {
            File::open(config_path)?
        } else {
            let file = File::create(config_path)?;
            serde_json::to_writer_pretty(&file, &Config::default())?;
            file
        }
    };
    match serde_json::from_reader(&config_file) {
        Ok(config) => Ok(config),
        Err(e) => Err(std::io::Error::from(e)),
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub address: String,
    pub port: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config { address: "127.0.0.1".to_string(), port: 8080 }
    }
}