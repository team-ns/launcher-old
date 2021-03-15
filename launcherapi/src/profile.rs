use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub version: String,
    pub libraries: Vec<String>,
    pub class_path: Vec<String>,
    pub main_class: String,
    pub update_verify: Vec<String>,
    pub update_exclusion: Vec<String>,
    pub jvm_args: Vec<String>,
    pub client_args: Vec<String>,
    pub assets: String,
    pub assets_dir: String,
    pub server_name: String,
    pub server_port: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProfileInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}
