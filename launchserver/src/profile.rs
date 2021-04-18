use anyhow::Result;
use launcher_api::optional::Optional;
use launcher_api::profile::{Profile, ProfileData, ProfileInfo};
use log::{error, warn};
use serde::de;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

pub static BLACK_LIST: [&str; 2] = ["profile.json", "description.txt"];

pub struct ProfileService {
    pub profiles_data: HashMap<String, ProfileData>,
}

#[teloc::inject]
impl ProfileService {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ProfileService {
    fn default() -> Self {
        Self {
            profiles_data: get_profiles_data(),
        }
    }
}

pub fn get_profiles_data() -> HashMap<String, ProfileData> {
    let mut profiles_data = HashMap::new();

    let profile_iter = WalkDir::new("static/profiles")
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .flat_map(|v| v.ok())
        .filter(|e| e.metadata().map(|m| m.is_dir()).unwrap_or(false));

    for profile_dir in profile_iter {
        match get_from_entry::<Profile, _>(&profile_dir.path().join("profile.json")) {
            Ok(profile) => {
                let profile_info = ProfileInfo {
                    name: profile.name.clone(),
                    version: profile.version.clone(),
                    description: get_description(&profile, &profile_dir),
                    optionals: get_optionals(&profile_dir),
                };
                profiles_data.insert(
                    profile.name.clone(),
                    ProfileData {
                        profile,
                        profile_info,
                    },
                );
            }
            Err(error) => {
                error!(
                    "Failed to read profile {:?}: {}",
                    &profile_dir.file_name(),
                    error
                );
            }
        }
    }
    profiles_data
}

fn get_optionals(profile_dir: &DirEntry) -> Vec<Optional> {
    let path = profile_dir.path().join("optionals.json");
    if path.exists() {
        match get_from_entry::<Vec<Optional>, _>(path) {
            Ok(mut optionals) => {
                optionals.retain(|opt| {
                    let visible_none = opt.visible && opt.name.is_none();
                    if visible_none {
                        error!(
                            "Find visible optional without name in profile `{:?}`",
                            profile_dir.file_name()
                        )
                    }
                    if opt.enabled && !opt.visible && (opt.name.is_some() || opt.description.is_some()) {
                        warn!("Find useless name or description for invisible optional: `{}` in profile: `{:?}`",
                              opt.name.as_ref().unwrap(),
                              profile_dir.file_name())
                    }

                    !visible_none
                });
                optionals.dedup_by(|a, b| {
                    let unique = a.name.is_some() && b.name.is_some() && a.name == b.name;
                    if !unique {
                        error!(
                            "Find duplicate name for optional: `{}` in profile: `{:?}`",
                            a.name.as_ref().unwrap(),
                            profile_dir.file_name()
                        )
                    }
                    unique
                });
                optionals
            }
            Err(error) => {
                error!(
                    "Failed to read optionals for profile {:?}: {}",
                    profile_dir.file_name(),
                    error
                );
                vec![]
            }
        }
    } else {
        vec![]
    }
}

fn get_from_entry<T: de::DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T> {
    let file = File::open(path)?;
    serde_json::from_reader::<_, T>(file).map_err(|error| anyhow::anyhow!(error))
}

fn get_description(profile: &Profile, entry: &DirEntry) -> String {
    fs::read_to_string(entry.path().join("description.txt")).unwrap_or(format!(
        "Minecraft server\n\
                         Version: {}\n\
                         Name:{}",
        &profile.version, &profile.name
    ))
}
