use crate::config::Config;
use crate::profile::ProfileService;
use crate::{profile, LauncherServiceProvider};
use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use launcher_api::profile::Profile;
use launcher_api::validation::{OsType, RemoteDirectory, RemoteFile};
use log::{error, info};
use path_slash::{PathBufExt, PathExt};
use reqwest::Url;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use teloc::Resolver;
use tokio::sync::RwLock;
use walkdir::{DirEntry, WalkDir};

#[derive(PartialEq, Eq, Hash)]
pub struct NativeVersion {
    pub version: String,
    pub os_type: OsType,
}

pub struct HashingService {
    pub profiles: Option<HashMap<String, RemoteDirectory>>,
    pub libraries: Option<HashMap<String, RemoteDirectory>>,
    pub assets: Option<HashMap<String, RemoteDirectory>>,
    pub natives: Option<HashMap<NativeVersion, RemoteDirectory>>,
    pub jres: Option<HashMap<OsType, RemoteDirectory>>,
}

#[teloc::inject]
impl HashingService {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for HashingService {
    fn default() -> Self {
        Self {
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

impl HashingService {
    pub fn rehash<'a, I: Clone + Iterator<Item = &'a Profile>>(
        &mut self,
        profiles: I,
        args: &[&str],
        file_server: String,
    ) {
        get_resource!(
            args,
            self.profiles,
            Self::hash_profiles(profiles.clone(), file_server.clone())
        );
        get_resource!(
            args,
            self.libraries,
            Self::hash_libraries(profiles, file_server.clone())
        );
        get_resource!(args, self.assets, Self::hash_assets(file_server.clone()));
        get_resource!(args, self.natives, Self::hash_natives(file_server.clone()));
        get_resource!(args, self.jres, Self::hash_jres(file_server));
        info!("Rehash was successfully finished!");
    }

    fn hash_profiles<'a, I: Clone + Iterator<Item = &'a Profile>>(
        profiles: I,
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

    fn hash_libraries<'a, I: Clone + Iterator<Item = &'a Profile>>(
        profiles: I,
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
                (OsType::MacOsX64, RemoteDirectory::new()),
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
                        os_type = Some(OsType::MacOsX64);
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
            (OsType::MacOsX64, "MacOSX64"),
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
}

fn strip_folder(path: &Path, save_number: usize, skip_number: usize) -> String {
    path.iter()
        .take(save_number)
        .chain(path.iter().skip(save_number + skip_number))
        .collect::<PathBuf>()
        .to_slash_lossy()
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

pub async fn rehash(sp: Arc<LauncherServiceProvider>, args: &[&str]) {
    let config: &Config = sp.resolve();
    let profile_service: Arc<RwLock<ProfileService>> = sp.resolve();
    let profile_service = profile_service.read().await;
    let hashing_service: Arc<RwLock<HashingService>> = sp.resolve();
    let mut hashing_service = hashing_service.write().await;
    hashing_service.rehash(
        profile_service
            .profiles_data
            .values()
            .map(|data| &data.profile),
        args,
        config.file_server.clone(),
    );
}
