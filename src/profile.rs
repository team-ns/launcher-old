use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
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