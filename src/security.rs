use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use ecies_ed25519::SecretKey;
use launcher_api::profile::Profile;
use launcher_api::validation::{HashedDirectory, HashedFile, OsType};
use log::{error, info};
use rand::rngs::OsRng;
use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

#[derive(PartialEq, Eq, Hash)]
pub struct NativeVersion {
    pub(crate) version: String,
    pub(crate) os_type: OsType,
}

pub struct SecurityManager {
    pub secret_key: SecretKey,
    pub profiles: Option<HashMap<String, HashedDirectory>>,
    pub assets: Option<HashMap<String, HashedDirectory>>,
    pub natives: Option<HashMap<NativeVersion, HashedDirectory>>,
    pub jres: Option<HashMap<OsType, HashedDirectory>>,
}

impl Default for SecurityManager {
    fn default() -> Self {
        let public_key = Path::new("public_key");
        let secret_key = Path::new("secret_key");
        if !public_key.exists() || !secret_key.exists() {
            info!("Creating new EC KeyPair...");
            SecurityManager::create_keys(public_key, secret_key).expect("Failed to create keys!");
        }
        let mut bytes = Vec::new();
        File::open("secret_key")
            .expect("Failed to get secret_key, try restart launch_server!")
            .read_to_end(&mut bytes)
            .expect("Failed to read secret_key, try delete it and restart launch_server!");
        SecurityManager {
            secret_key: SecretKey::from_bytes(&bytes).expect("Failed to parse key!"),
            profiles: None,
            assets: None,
            natives: None,
            jres: None,
        }
    }
}

macro_rules! get_resource {
    ($args:expr, $resource:expr, $hash_resource:expr) => {
        let resource_name = &stringify!($resource)[5..];
        if $args.is_empty() || $args.contains(&resource_name) {
            match $hash_resource {
                Ok(resource) => {
                    $resource = Some(resource);
                    info!("Successfully rehash {}!", resource_name);
                }
                Err(error) => error!("Error while hashing {}: {}!", resource_name, error),
            }
        }
    };
}

impl SecurityManager {
    pub fn decrypt(&self, text: &str) -> Result<String, String> {
        let text = base64::decode(text).map_err(|e| format!("Can't decode base64: {:?}!", e))?;
        let pwd = ecies_ed25519::decrypt(&self.secret_key, &text)
            .map_err(|e| format!("Invalid encrypted password: {:?}!", e))?;
        Ok(String::from_utf8(pwd).map_err(|_| "Password contains invalid symbols!")?)
    }

    fn create_keys(public_key: &Path, secret_key: &Path) -> Result<()> {
        let (secret, public) = ecies_ed25519::generate_keypair(&mut OsRng);
        SecurityManager::create_key(public_key, public.to_bytes())?;
        SecurityManager::create_key(secret_key, secret.to_bytes())?;
        Ok(())
    }

    fn create_key(path: &Path, bytes: [u8; 32]) -> Result<()> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)?
            .write_all(&bytes)?;
        Ok(())
    }

    pub fn rehash(&mut self, profiles: Values<String, Profile>, args: &[&str]) {
        get_resource!(
            args,
            self.profiles,
            SecurityManager::hash_profiles(profiles)
        );
        get_resource!(args, self.assets, SecurityManager::hash_assets());
        get_resource!(args, self.natives, SecurityManager::hash_natives());
        get_resource!(args, self.jres, SecurityManager::hash_jres());
        info!("Rehash was successfully finished!");
    }

    fn hash_profiles(
        profiles: Values<String, Profile>,
    ) -> Result<HashMap<String, HashedDirectory>> {
        let mut hashed_profiles = HashMap::new();
        let hashed_libs = create_hashed_dir("static/libs")?;

        for profile in profiles {
            let mut hashed_profile = HashedDirectory::new();
            let black_list = vec!["profile.json", "description.txt"];

            let file_iter = get_files_from_dir(format!("static/profiles/{}", profile.name))
                .filter(|e| !black_list.contains(&e.file_name().to_str().unwrap_or("")));
            fill_map(file_iter, &mut hashed_profile)?;

            for lib in &profile.libraries {
                let lib = format!("libs/{}", lib);
                match hashed_libs.get(&lib) {
                    Some(file) => {
                        hashed_profile.insert(lib.clone(), file.clone());
                    }
                    None => {
                        error!(
                            "Profile '{}' use lib '{}' that doesn't exists in files!",
                            profile.name, lib
                        );
                    }
                }
            }
            hashed_profiles.insert(profile.name.clone(), hashed_profile);
        }
        Ok(hashed_profiles)
    }

    fn hash_assets() -> Result<HashMap<String, HashedDirectory>> {
        let mut hashed_assets = HashMap::new();

        for version in get_first_level_dirs("static/assets") {
            let path = version.path();
            hashed_assets.insert(strip(path, "static/assets/")?, create_hashed_dir(path)?);
        }
        Ok(hashed_assets)
    }

    fn hash_natives() -> Result<HashMap<NativeVersion, HashedDirectory>> {
        let mut hashed_natives: HashMap<NativeVersion, HashedDirectory> = HashMap::new();

        for version in get_first_level_dirs("static/natives") {
            let mut hashed_native: HashMap<OsType, HashedDirectory> = [
                (OsType::LinuxX64, HashedDirectory::new()),
                (OsType::LinuxX32, HashedDirectory::new()),
                (OsType::MacOSX64, HashedDirectory::new()),
                (OsType::WindowsX64, HashedDirectory::new()),
                (OsType::WindowsX32, HashedDirectory::new()),
            ]
            .iter()
            .cloned()
            .collect();

            let version_path = version.path();
            for file in get_files_from_dir(version_path) {
                let path = file.path();

                let extension = path.extension().and_then(OsStr::to_str);
                if extension.is_some() {
                    let mut os_type = None;
                    let extension = extension.unwrap();
                    if extension.eq("dll") {
                        let mut file = File::open(path)?;
                        file.seek(SeekFrom::Start(0x3C))?;
                        let pe_header = file.read_u32::<LittleEndian>()?;
                        file.seek(SeekFrom::Start((pe_header + 4) as u64))?;
                        let arch = file.read_u16::<LittleEndian>()?;

                        if arch == 0x014c {
                            os_type = Some(OsType::WindowsX32);
                        } else if arch == 0x8664 {
                            os_type = Some(OsType::WindowsX64);
                        }
                    } else if extension.eq("so") {
                        let mut file = File::open(path)?;
                        file.seek(SeekFrom::Start(4))?;
                        let arch = file.read_u8()?;
                        if arch == 1 {
                            os_type = Some(OsType::LinuxX32);
                        } else if arch == 2 {
                            os_type = Some(OsType::LinuxX64);
                        }
                    } else if extension.eq("dylib") || extension.eq("jnilib") {
                        os_type = Some(OsType::MacOSX64);
                    } else {
                        error!("Found excess file: {:?} in native dir!", path);
                        continue;
                    }
                    match os_type {
                        Some(os_type) => {
                            hashed_native.get_mut(&os_type).unwrap().insert(
                                strip(path, "static/")?,
                                HashedFile::new(path.to_string_lossy().as_ref()),
                            );
                        }
                        None => error!("Unknown file archetype: {:?}!", path),
                    }
                } else {
                    error!("Cannot get file: {:?} extension!", path);
                }
            }

            let version = strip(version_path, "static/natives/")?;

            for native in hashed_native {
                let native_version = NativeVersion {
                    version: version.clone(),
                    os_type: native.0.to_owned(),
                };
                hashed_natives.insert(native_version, native.1);
            }
        }
        Ok(hashed_natives)
    }

    fn hash_jres() -> Result<HashMap<OsType, HashedDirectory>> {
        let mut hashed_jres = HashMap::new();

        let jres = vec![
            (OsType::LinuxX64, "LinuxX64"),
            (OsType::LinuxX32, "LinuxX32"),
            (OsType::MacOSX64, "MacOSX64"),
            (OsType::WindowsX64, "WindowsX64"),
            (OsType::WindowsX32, "WindowsX32"),
        ];

        for jre in jres {
            hashed_jres.insert(jre.0, create_hashed_dir(format!("static/jre/{}", jre.1))?);
        }
        Ok(hashed_jres)
    }
}

fn fill_map(
    iter: impl Iterator<Item = DirEntry>,
    map: &mut HashMap<String, HashedFile>,
) -> Result<()> {
    for file in iter {
        let path = file.path();
        map.insert(
            strip(path, "static/")?,
            HashedFile::new(path.to_string_lossy().as_ref()),
        );
    }
    Ok(())
}

fn create_hashed_dir<P: AsRef<Path>>(path: P) -> Result<HashedDirectory> {
    let mut directory = HashedDirectory::new();
    let iter = get_files_from_dir(path);
    fill_map(iter, &mut directory)?;
    Ok(directory)
}

fn get_files_from_dir<P: AsRef<Path>>(path: P) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(path)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.metadata().map(|m| m.is_file()).unwrap_or(false))
}

fn get_first_level_dirs<P: AsRef<Path>>(path: P) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.metadata().map(|m| m.is_dir()).unwrap_or(false))
}

fn strip(path: &Path, prefix: &str) -> Result<String> {
    Ok(path
        .strip_prefix(prefix)?
        .to_str()
        .with_context(|| {
            format!(
                "Can't strip prefix for path {:?}, maybe it is have non unicode chars!",
                path
            )
        })?
        .to_string())
}
