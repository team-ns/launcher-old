use launcher_api::profile::{Profile, ProfileInfo};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use walkdir::WalkDir;

pub fn get_profiles() -> (HashMap<String, Profile>, Vec<ProfileInfo>) {
    let profiles: HashMap<String, Profile> = WalkDir::new("static/profiles")
        .min_depth(2)
        .max_depth(3)
        .into_iter()
        .flat_map(|v| v.ok())
        .filter(|e| {
            e.metadata().map(|m| m.is_file()).unwrap_or(false) && e.file_name().eq("profile.json")
        })
        .flat_map(|e| File::open(e.into_path()).ok())
        .flat_map(|f| serde_json::from_reader::<File, Profile>(f).ok())
        .map(|profile| (profile.name.clone(), profile))
        .collect();
    let profiles_info = profiles
        .values()
        .map(|profile| {
            let description =
                fs::read_to_string(format!("static/profiles/{}/description.txt", &profile.name))
                    .unwrap_or(format!(
                        "Minecraft server\n\
                         Version: {}\n\
                         Name:{}",
                        &profile.version, &profile.name
                    ));
            ProfileInfo {
                name: profile.name.clone(),
                version: profile.version.clone(),
                description,
            }
        })
        .collect();
    (profiles, profiles_info)
}
