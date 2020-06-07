use std::collections::HashMap;
use walkdir::WalkDir;

#[derive(Clone)]
pub struct HashedFile {
    pub len: u64,
    pub checksum: u128,
}

/*pub struct HashedProfile {
    pub name: String,
    pub libraries: Vec<HashedFile>,
    pub classpath: Vec<HashedFile>,
    pub mods: Vec<HashedFile>,
}*/

pub type HashedProfile = HashMap<String, HashedFile>;

pub fn get_profiles() -> Vec<String> {
    WalkDir::new("static")
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter(|e| e.is_ok() && e.as_ref().ok().unwrap().metadata().unwrap().is_dir())
        .map(|e| String::from(e.ok().unwrap().file_name().to_str().unwrap()))
        .collect()
}
