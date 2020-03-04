use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    name: String,
    version: String,
    libraries: Vec<String>,
    class_path: Vec<String>,
    main_class: String,
    jvm_args: Vec<String>,
    client_args: Vec<String>,
    assets: String,
    assets_dir: String,
}