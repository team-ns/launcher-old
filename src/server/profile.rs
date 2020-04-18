use walkdir::WalkDir;

pub fn get_profiles() -> Vec<String> {
    WalkDir::new("static")
        .min_depth(1).max_depth(1)
        .into_iter()
        .filter(|e| e.is_ok() && e.as_ref().ok().unwrap().metadata().unwrap().is_dir())
        .map(|e| String::from(e.ok().unwrap().file_name().to_str().unwrap()))
        .collect()
}