use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LauncherBundle {
    pub game_dir: String,
    pub websocket: String,
    pub ram: u64,
    pub project_name: String,
    pub public_key: [u8; 32],
    pub window: Window,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Window {
    pub frameless: bool,
    pub resizable: bool,
    pub transparent: bool,
    pub width: u32,
    pub height: u32,
}
