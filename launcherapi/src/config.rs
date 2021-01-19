use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::fs::OpenOptions;
use std::path::Path;

pub trait Configurable: Default + Serialize + DeserializeOwned {
    fn get_config(config_path: &Path) -> Result<Self> {
        if let Some(path) = config_path.parent() {
            fs::create_dir_all(path)?;
        }

        if !config_path.exists() {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(config_path)?;

            let config = Self::default();
            serde_json::to_writer_pretty(&file, &config)?;
            return Ok(config);
        }

        let config_file = OpenOptions::new().read(true).open(config_path)?;
        serde_json::from_reader(&config_file).map_err(|error| anyhow::anyhow!(error))
    }
}
