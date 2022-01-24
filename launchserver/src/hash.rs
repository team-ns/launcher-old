use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use log::{error, info, warn};
use path_slash::PathExt;
use reqwest::Url;
use teloc::Resolver;
use tokio::sync::RwLock;
use walkdir::DirEntry;

use futures::{future, Stream, StreamExt};
use itertools::Itertools;
use launcher_api::profile::ProfileData;
use launcher_api::validation::{OsType, RemoteDirectory, RemoteFile};
use launcher_macro::hash;

use tokio::fs;

use crate::config::Config;
use crate::hash::resources::{FileLocation, NativeVersion};
use crate::profile::ProfileService;
use crate::util;
use crate::LauncherServiceProvider;

mod arch;
pub mod resources;

#[derive(Debug, Default)]
pub struct HashingService {
    pub files: HashMap<FileLocation, RemoteDirectory>,
}

#[teloc::inject]
impl HashingService {
    pub fn new() -> Self {
        Self::default()
    }
}

impl HashingService {
    pub async fn rehash(
        &mut self,
        args: &[&str],
        file_server: &str,
        profiles_data: Vec<&ProfileData>,
    ) {
        for dir in &["profiles", "libraries", "natives", "jre"] {
            let path = Path::new("static").join(dir);
            if !path.exists() {
                match fs::create_dir_all(path).await {
                    Ok(_) => info!("Create empty directory for {}", dir),
                    Err(error) => error!("Failed to create empty directory for {}: {}", dir, error),
                }
            }
        }
        hash!(
            args,
            self.hash_profiles(file_server, &profiles_data),
            self.hash_libraries(file_server, &profiles_data),
            self.hash_assets(file_server),
            self.hash_natives(file_server),
            self.hash_jres(file_server)
        );
        info!("Rehash was successfully finished!");
    }

    async fn hash_profiles(&mut self, file_server: &str, profiles_data: &[&ProfileData]) {
        for profile_data in profiles_data {
            let profile = &profile_data.profile.name;
            let remote_directory =
                Self::get_remote_dir(Path::new("static/profiles").join(profile), file_server).await;
            self.files
                .insert(FileLocation::Profile(profile.to_string()), remote_directory);
        }
    }

    async fn hash_libraries(&mut self, file_server: &str, profiles_data: &[&ProfileData]) {
        let remote_directory =
            Self::get_remote_dir(Path::new("static/libraries"), file_server).await;

        for profile_data in profiles_data {
            let mut profile_libs = RemoteDirectory::new();
            let profile = &profile_data.profile;

            let rename_paths = profile_data
                .profile_info
                .optionals
                .iter()
                .flat_map(|optional| optional.get_paths())
                .collect::<Vec<_>>();

            for lib in &profile.libraries {
                let lib_path = PathBuf::from("libraries").join(lib);

                match remote_directory.get(&lib_path) {
                    Some(file) => {
                        profile_libs.insert(lib_path, file.clone());
                    }
                    None => {
                        let paths = rename_paths
                            .iter()
                            .filter_map(|(key, val)| {
                                if val == &&PathBuf::from(lib) {
                                    Some(key)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
                        if paths.is_empty() {
                            error!(
                                "Profile '{}' use lib '{:?}' that doesn't exists in files!",
                                profile.name, lib_path
                            );
                        } else {
                            for path in paths {
                                let lib_path = PathBuf::from("libraries").join(path);
                                match remote_directory.get(&lib_path) {
                                    Some(file) => {
                                        profile_libs.insert(lib_path, file.clone());
                                    }
                                    None => {
                                        error!(
                                             "Profile '{}' use optionals file action for renaming lib {:?} that doesn't exists in files!",
                                             profile.name, path
                                         );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            self.files
                .insert(FileLocation::Libraries(profile.name.clone()), profile_libs);
        }
    }

    async fn hash_assets(&mut self, file_server: &str) {
        for version in util::fs::get_first_level_dirs("static/assets") {
            let version_path = version.path();
            match util::fs::strip(version_path, "static/assets/") {
                Ok(path) => {
                    self.files.insert(
                        FileLocation::Assets(path),
                        Self::get_remote_dir(version_path, file_server).await,
                    );
                }
                Err(error) => error!("Failed to get version while hashing assets: {:?}", error),
            }
        }
    }

    async fn hash_natives(&mut self, file_server: &str) {
        for version in util::fs::get_first_level_dirs("static/natives") {
            let version_path = version.path();
            let hashed_native =
                Self::get_hash_stream(util::fs::get_files_from_dir(version_path), file_server, &Ok)
                    .filter_map(|file| async {
                        match arch::get_os_type(&file.0).await {
                            Ok(os_type) => match file.0.strip_prefix("static/") {
                                Ok(path) => Some((os_type, (PathBuf::from(path), file.1))),
                                Err(error) => {
                                    error!("Failed strip native path: {}", error);
                                    None
                                }
                            },
                            Err(error) => {
                                error!("Error while hashing natives: {}", error);
                                None
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .into_group_map();

            match util::fs::strip(version_path, "static/natives/") {
                Ok(version) => {
                    for native in hashed_native {
                        let native_version = NativeVersion::new(version.clone(), native.0);
                        self.files.insert(
                            FileLocation::Natives(native_version),
                            native.1.into_iter().collect(),
                        );
                    }
                }
                Err(error) => {
                    error!("{:?}", error);
                }
            }
        }
    }

    async fn hash_jres(&mut self, file_server: &str) {
        let types = vec![
            OsType::LinuxX64,
            OsType::LinuxX32,
            OsType::MacOsX64,
            OsType::WindowsX64,
            OsType::WindowsX32,
        ];

        for dir in util::fs::get_first_level_dirs(Path::new("static/jre")) {
            for os_type in &types {
                let jre_dir = dir.path().join(os_type.to_string());
                if jre_dir.exists() {
                    let remote_directory = Self::get_hash_stream(
                        util::fs::get_files_from_dir(jre_dir),
                        file_server,
                        &|path: PathBuf| {
                            path.strip_prefix("static/")
                                .map(|path| util::fs::strip_folder(path, 2, 1))
                                .map_err(|error| anyhow::anyhow!(error))
                        },
                    )
                    .collect::<HashMap<_, _>>()
                    .await;
                    match dir.file_name().to_str() {
                        Some(name) => {
                            self.files.insert(
                                FileLocation::Jres(name.to_string(), os_type.clone()),
                                remote_directory,
                            );
                        }
                        None => error!("Failed get JRE dir name"),
                    }
                } else {
                    warn!("No such JRE {:?} for os type: {}", dir.file_name(), os_type);
                }
            }
        }
    }

    fn get_hash_stream<'a, I, F>(
        files: I,
        file_server: &'a str,
        strip: &'a F,
    ) -> impl Stream<Item = (PathBuf, RemoteFile)> + 'a
    where
        F: Fn(PathBuf) -> Result<PathBuf>,
        I: Iterator<Item = DirEntry> + 'a,
    {
        futures::stream::iter(files.map(|e| e.into_path()))
            .map(move |path| async move {
                match Self::get_remote_file(file_server, path.as_path()).await {
                    Ok(file) => match strip(path) {
                        Ok(path) => Some((path, file)),
                        Err(error) => {
                            error!("Failed get file path: {:?}", error);
                            None
                        }
                    },
                    Err(error) => {
                        error!("Error while hashing: {:?}", error);
                        None
                    }
                }
            })
            .buffer_unordered(50)
            .filter_map(future::ready)
    }

    async fn get_remote_file<P: AsRef<Path>>(file_server: &str, path: P) -> Result<RemoteFile> {
        match &fs::read(path.as_ref()).await {
            Ok(bytes) => match Url::parse(&format!(
                "{}/{}",
                file_server,
                path.as_ref()
                    .strip_prefix("static/")
                    .expect("Failed to strip prefix to create remote file!")
                    .to_slash_lossy()
            )) {
                Ok(uri) => Ok(RemoteFile {
                    uri: uri.to_string(),
                    size: bytes.len(),
                    checksum: t1ha::t1ha2_atonce128(bytes, 1),
                }),
                Err(error) => Err(anyhow::anyhow!(
                    "Failed to parse uri for file {:?} with error {:?}",
                    path.as_ref(),
                    error
                )),
            },
            Err(error) => Err(anyhow::anyhow!(
                "Failed to read file {:?} with error {:?}",
                path.as_ref(),
                error
            )),
        }
    }

    async fn get_remote_dir<P: AsRef<Path>>(path: P, file_server: &str) -> RemoteDirectory {
        Self::get_hash_stream(
            util::fs::get_files_from_dir(path),
            file_server,
            &|path: PathBuf| util::fs::strip(path.as_path(), "static/").map(PathBuf::from),
        )
        .collect::<HashMap<_, _>>()
        .await
    }
}

pub async fn rehash(sp: Arc<LauncherServiceProvider>, args: &[&str]) {
    let config: &Config = sp.resolve();
    let profile_service: Arc<RwLock<ProfileService>> = sp.resolve();
    let profile_service = profile_service.read().await;
    let hashing_service: Arc<RwLock<HashingService>> = sp.resolve();
    let mut hashing_service = hashing_service.write().await;
    hashing_service
        .rehash(
            args,
            &config.file_server,
            profile_service.profiles_data.values().collect::<Vec<_>>(),
        )
        .await;
}
