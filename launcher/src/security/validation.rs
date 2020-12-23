use anyhow::Result;
use launcher_api::message::ProfileResourcesResponse;
use launcher_api::validation::{HashedDirectory, HashedFile, OsType, RemoteFile};
use std::path::PathBuf;
use web_view::Handle;

use crate::client::downloader;

pub enum ValidationStatus {
    Success,
    NeedUpdate(Vec<RemoteFile>),
}

fn resource_exists(game_dir: &str, resource: &str) -> bool {
    PathBuf::from(format!("{}/{}/", game_dir, resource))
        .read_dir()
        .map(|mut dir| dir.next().is_some())
        .unwrap_or(false)
}

macro_rules! extend {
    ($files:expr, $game_dir:expr, $resource:expr) => {
        $files.extend($resource);
    };
    ($files:expr, $game_dir:expr, $resource:expr, $($resources:expr),+) => {
        extend!($files, $game_dir, $resource);
        extend!($files, $game_dir, $($resources),+);
    };
}

macro_rules! check_resources {
    ($file_server:expr, $handler:expr, $game_dir:expr, $($resources:expr),+) => {
        let mut files = HashedDirectory::new();
        exists!(files, $game_dir, $($resources),+);
        let remote_files = profile_into_remote(files.iter());
        downloader::download(remote_files, $file_server, $handler).await?;
    };
}

macro_rules! exists {
    ($files:expr, $game_dir:expr, $resource:expr) => {
        if !resource_exists($game_dir, &stringify!($resource)[10..]) {
            $files.extend($resource);
        }
    };
    ($files:expr, $game_dir:expr, $resource:expr, $($resources:expr),+) => {
        exists!($files, $game_dir, $resource);
        exists!($files, $game_dir, $($resources),+);
    };
}

pub async fn validate_profile(
    game_dir: String,
    _profile_name: String,
    resources: ProfileResourcesResponse,
    file_server: String,
    handler: Handle<()>,
) -> Result<()> {
    let mut files = HashedDirectory::new();
    extend!(
        files,
        resources.profile.clone(),
        resources.libraries.clone(),
        resources.assets.clone(),
        resources.natives.clone(),
        resources.jre.clone()
    );

    check_resources!(
        file_server.clone(),
        handler.clone(),
        &game_dir,
        resources.profile,
        resources.libraries,
        resources.assets,
        resources.natives,
        resources.jre
    );

    //watcher start
    match validate(&files, game_dir.clone())? {
        ValidationStatus::NeedUpdate(files_to_update) => {
            //watcher stop
            downloader::download(files_to_update, file_server, handler).await?;
            //watcher start
            match validate(&files, game_dir)? {
                ValidationStatus::Success => Ok(()),
                ValidationStatus::NeedUpdate(files) => Err(anyhow::anyhow!(
                    "Sync error: {:?}",
                    files
                        .into_iter()
                        .take(5)
                        .map(|file| file.name)
                        .collect::<Vec<_>>()
                )),
            }
        }
        ValidationStatus::Success => Ok(()),
    }
}

fn validate(profile: &HashedDirectory, game_dir: String) -> Result<ValidationStatus> {
    let profile = profile.iter().filter(|file| {
        HashedFile::new(format!("{}/{}", game_dir, downloader::get_path(file.0)))
            .map_or(false, |ref hashed_file| hashed_file == file.1)
    });
    let remote = profile_into_remote(profile);
    if remote.is_empty() {
        Ok(ValidationStatus::Success)
    } else {
        Ok(ValidationStatus::NeedUpdate(remote))
    }
}

fn profile_into_remote<'a>(
    iter: impl Iterator<Item = (&'a String, &'a HashedFile)>,
) -> Vec<RemoteFile> {
    iter.map(|file| RemoteFile {
        name: file.0.clone(),
        size: file.1.size,
    })
    .collect()
}

pub fn get_os_type() -> OsType {
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let os_type = OsType::MacOSX64;
    #[cfg(all(target_os = "linux"))]
    let os_type = {
        use uname;
        let info = uname::uname().expect("Can't get os info");

        Ok(match info.machine.as_ref() {
            "i686" => OsType::LinuxX32,
            "x86_64" => OsType::LinuxX64,
            _ => unreachable!(),
        })
    };
    #[cfg(all(target_os = "windows"))]
    let os_type = {
        use std::mem;
        use winapi::um::sysinfoapi::{GetNativeSystemInfo, SYSTEM_INFO_u_s, SYSTEM_INFO};
        use winapi::um::winnt::{PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_INTEL};

        let mut system_info: SYSTEM_INFO = unsafe { mem::zeroed() };

        unsafe { GetNativeSystemInfo(&mut system_info) };

        let s: &SYSTEM_INFO_u_s = unsafe { system_info.u.s() };

        match s.wProcessorArchitecture {
            PROCESSOR_ARCHITECTURE_INTEL => OsType::WindowsX32,
            PROCESSOR_ARCHITECTURE_AMD64 => OsType::WindowsX64,
            _ => unreachable!(),
        }
    };
    os_type
}
