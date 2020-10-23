use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub version: String,
    pub libraries: Vec<String>,
    pub class_path: Vec<String>,
    pub main_class: String,
    pub jvm_args: Vec<String>,
    pub client_args: Vec<String>,
    pub assets: String,
    pub assets_dir: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ProfileInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}
