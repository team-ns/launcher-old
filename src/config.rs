use std::path::Path;
use std::fs::OpenOptions;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub trait Configurable: Default + Serialize + DeserializeOwned {
    fn get_config(config_path: &Path) -> std::io::Result<Self> {
        let config_file = OpenOptions::new()
            .write(true)
            .create(true)
            .read(true)
            .open(config_path)?;
        match serde_json::from_reader(&config_file) {
            Ok(config) => Ok(config),
            Err(_e) => {
                let config = Self::default();
                serde_json::to_writer_pretty(&config_file, &config)?;
                Ok(config)
            },
        }
    }
}