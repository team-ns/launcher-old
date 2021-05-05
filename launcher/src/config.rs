use anyhow::Result;

use launcher_api::bundle::LauncherBundle;
use once_cell::sync::{Lazy, OnceCell};
use path_slash::PathExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub static BUNDLE: Lazy<LauncherBundle> = Lazy::new(load_bundle);

pub static SETTINGS: OnceCell<Arc<Mutex<Settings>>> = OnceCell::new();

#[cfg(feature = "bundle")]
fn load_bundle() -> LauncherBundle {
    let bundle = include_crypt::include_crypt!("bundle.bin").decrypt();
    let bundle = {
        let mut bundle =
            bincode::deserialize::<LauncherBundle>(&bundle).expect("Can't read bundle");
        bundle.game_dir = bundle
            .game_dir
            .replace("%homeDir%", &dirs::home_dir().unwrap().to_slash_lossy());
        bundle
    };
    bundle
}

#[cfg(not(feature = "bundle"))]
fn load_bundle() -> LauncherBundle {
    let config_json = fs::read_to_string("config.json")
        .expect("Can't find configuration")
        .replace("%homeDir%", &dirs::home_dir().unwrap().to_slash_lossy());
    serde_json::from_str(&config_json).unwrap()
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub game_dir: String,
    pub save_data: bool,
    pub ram: u64,
    pub saved_password: Option<String>,
    pub last_name: Option<String>,
    pub optionals: HashMap<String, Vec<String>>,
    pub properties: HashMap<String, String>,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let path = Path::new(&BUNDLE.game_dir).join("settings.bin");
        fs::create_dir_all(path.parent().unwrap())?;
        let settings = bincode::deserialize::<Self>(&fs::read(path)?)?;
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let body = bincode::serialize(self)?;
        let path = Path::new(&BUNDLE.game_dir).join("settings.bin");
        let mut file = File::create(path)?;
        file.write_all(&body)?;
        Ok(())
    }

    pub fn update(&mut self, settings: &Self) -> Result<()> {
        self.ram = settings.ram;
        Ok(())
    }

    pub fn get_optionals(&self, profile: &str) -> Vec<String> {
        self.optionals
            .get(profile)
            .map(Clone::clone)
            .unwrap_or_default()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            game_dir: BUNDLE.game_dir.clone(),
            save_data: false,
            ram: BUNDLE.ram,
            saved_password: None,
            last_name: None,
            optionals: Default::default(),
            properties: Default::default(),
        }
    }
}
