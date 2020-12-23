use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::fs::OpenOptions;
use std::path::Path;

pub trait Configurable: Default + Serialize + DeserializeOwned {
    fn get_config(config_path: &Path) -> std::io::Result<Self> {
        fs::create_dir_all(config_path.parent().unwrap())?;
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
            }
        }
    }
}
