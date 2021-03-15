use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use ecies_ed25519::SecretKey;
use log::{error, info};
use path_slash::PathExt;
use rand::rngs::OsRng;
use reqwest::Url;
use walkdir::DirEntry;

use crate::util::{get_files_from_dir, get_first_level_dirs, strip, strip_folder};
use launcher_api::profile::Profile;
use launcher_api::validation::{OsType, RemoteDirectory, RemoteFile};

use crate::server::profile;
use rand::Rng;

#[derive(PartialEq, Eq, Hash)]
pub struct NativeVersion {
    pub(crate) version: String,
    pub(crate) os_type: OsType,
}

pub struct SecurityManager {
    pub secret_key: SecretKey,
    pub profiles: Option<HashMap<String, RemoteDirectory>>,
    pub libraries: Option<HashMap<String, RemoteDirectory>>,
    pub assets: Option<HashMap<String, RemoteDirectory>>,
    pub natives: Option<HashMap<NativeVersion, RemoteDirectory>>,
    pub jres: Option<HashMap<OsType, RemoteDirectory>>,
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
            libraries: None,
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

pub fn create_remote_file<P: AsRef<Path>>(path: P, file_server: String) -> Result<RemoteFile> {
    let mut buffer = Vec::new();
    File::open(&path)?.read_to_end(&mut buffer)?;
    Ok(RemoteFile {
        uri: Url::parse(&format!(
            "{}/{}",
            file_server,
            path.as_ref()
                .strip_prefix("static/")
                .expect("Failed to strip prefix while rehash file!")
                .to_slash_lossy()
        ))?
        .into_string(),
        size: buffer.len() as u64,
        checksum: t1ha::t1ha2_atonce128(buffer.as_slice(), 1),
    })
}

impl SecurityManager {
    pub fn decrypt(&self, text: &str) -> Result<String> {
        let text =
            base64::decode(text).map_err(|e| anyhow::anyhow!("Can't decode base64: {:?}!", e))?;
        let pwd = ecies_ed25519::decrypt(&self.secret_key, &text)
            .map_err(|e| anyhow::anyhow!("Invalid encrypted password: {:?}!", e))?;
        Ok(String::from_utf8(pwd)
            .map_err(|_| anyhow::anyhow!("Password contains invalid symbols!"))?)
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

    pub fn rehash(
        &mut self,
        profiles: Values<String, Profile>,
        args: &[&str],
        file_server: String,
    ) {
        get_resource!(
            args,
            self.profiles,
            SecurityManager::hash_profiles(profiles.clone(), file_server.clone())
        );
        get_resource!(
            args,
            self.libraries,
            SecurityManager::hash_libraries(profiles, file_server.clone())
        );
        get_resource!(
            args,
            self.assets,
            SecurityManager::hash_assets(file_server.clone())
        );
        get_resource!(
            args,
            self.natives,
            SecurityManager::hash_natives(file_server.clone())
        );
        get_resource!(args, self.jres, SecurityManager::hash_jres(file_server));
        info!("Rehash was successfully finished!");
    }

    fn hash_profiles(
        profiles: Values<String, Profile>,
        file_server: String,
    ) -> Result<HashMap<String, RemoteDirectory>> {
        let mut hashed_profiles = HashMap::new();

        for profile in profiles {
            let mut hashed_profile = RemoteDirectory::new();

            let file_iter = get_files_from_dir(format!("static/profiles/{}", profile.name))
                .filter(|e| !profile::BLACK_LIST.contains(&e.file_name().to_str().unwrap_or("")));
            fill_map(file_iter, &mut hashed_profile, file_server.clone())?;

            hashed_profiles.insert(profile.name.clone(), hashed_profile);
        }
        Ok(hashed_profiles)
    }

    fn hash_libraries(
        profiles: Values<String, Profile>,
        file_server: String,
    ) -> Result<HashMap<String, RemoteDirectory>> {
        let mut libs = HashMap::new();
        for file in get_files_from_dir("static/libraries") {
            libs.insert(
                file.path().strip_prefix("static/")?.to_owned(),
                create_remote_file(file.path(), file_server.clone())?,
            );
        }
        let mut hashed_libs = HashMap::new();

        for profile in profiles {
            let mut hashed_profile_libs = RemoteDirectory::new();
            for lib in &profile.libraries {
                let lib = PathBuf::from(format!("libraries/{}", lib));
                match libs.get(&lib) {
                    Some(file) => {
                        hashed_profile_libs.insert(lib, file.clone());
                    }
                    None => {
                        error!(
                            "Profile '{}' use lib '{:?}' that doesn't exists in files!",
                            profile.name, lib
                        );
                    }
                }
            }
            hashed_libs.insert(profile.name.clone(), hashed_profile_libs);
        }
        Ok(hashed_libs)
    }

    fn hash_assets(file_server: String) -> Result<HashMap<String, RemoteDirectory>> {
        let mut hashed_assets = HashMap::new();

        for version in get_first_level_dirs("static/assets") {
            let path = version.path();
            hashed_assets.insert(
                strip(path, "static/assets/")?,
                create_hashed_dir(path, file_server.clone())?,
            );
        }
        Ok(hashed_assets)
    }

    fn hash_natives(file_server: String) -> Result<HashMap<NativeVersion, RemoteDirectory>> {
        let mut hashed_natives: HashMap<NativeVersion, RemoteDirectory> = HashMap::new();

        for version in get_first_level_dirs("static/natives") {
            let mut hashed_native: HashMap<OsType, RemoteDirectory> = [
                (OsType::LinuxX64, RemoteDirectory::new()),
                (OsType::LinuxX32, RemoteDirectory::new()),
                (OsType::MacOSX64, RemoteDirectory::new()),
                (OsType::WindowsX64, RemoteDirectory::new()),
                (OsType::WindowsX32, RemoteDirectory::new()),
            ]
            .iter()
            .cloned()
            .collect();

            let version_path = version.path();
            for file in get_files_from_dir(version_path) {
                let path = file.path();

                let extension = path.extension().and_then(OsStr::to_str);
                if let Some(extension) = extension {
                    let mut os_type = None;
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
                                PathBuf::from(strip(path, "static/")?),
                                create_remote_file(path, file_server.clone())?,
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

    fn hash_jres(file_server: String) -> Result<HashMap<OsType, RemoteDirectory>> {
        let mut hashed_jres = HashMap::new();

        let jres = vec![
            (OsType::LinuxX64, "LinuxX64"),
            (OsType::LinuxX32, "LinuxX32"),
            (OsType::MacOSX64, "MacOSX64"),
            (OsType::WindowsX64, "WindowsX64"),
            (OsType::WindowsX32, "WindowsX32"),
        ];

        for jre in jres {
            hashed_jres.insert(
                jre.0,
                create_hashed_dir(format!("static/jre/{}", jre.1), file_server.clone())?,
            );
        }
        Ok(hashed_jres)
    }

    pub fn create_access_token() -> String {
        let digest = {
            let mut rng = rand::thread_rng();
            md5::compute(format!(
                "{}{}{}",
                rng.gen_range(1000000000, 2147483647),
                rng.gen_range(1000000000, 2147483647),
                rng.gen_range(0, 9)
            ))
        };
        format!("{:x}", digest)
    }
}

fn fill_map(
    iter: impl Iterator<Item = DirEntry>,
    map: &mut HashMap<PathBuf, RemoteFile>,
    file_server: String,
) -> Result<()> {
    for file in iter {
        let path = file.path();
        let strip_path = if path.starts_with("static/jre") {
            strip_folder(
                path.strip_prefix("static/")
                    .expect("Failed to strip prefix!"),
                1,
                1,
            )
        } else {
            strip(path, "static/")?
        };
        map.insert(
            PathBuf::from(strip_path),
            create_remote_file(path, file_server.clone())?,
        );
    }
    Ok(())
}

fn create_hashed_dir<P: AsRef<Path>>(path: P, file_server: String) -> Result<RemoteDirectory> {
    let mut directory = RemoteDirectory::new();
    let iter = get_files_from_dir(path);
    fill_map(iter, &mut directory, file_server)?;
    Ok(directory)
}
