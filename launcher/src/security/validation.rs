use crate::client::downloader;
use crate::security::watcher::WatcherService;
use anyhow::Result;
use launcher_api::message::ProfileResourcesResponse;
use launcher_api::profile::Profile;
use launcher_api::validation::{HashedFile, OsType, RemoteDirectory, RemoteFile};
use log::debug;
use path_slash::PathExt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio::stream::StreamExt;
use web_view::Handle;

pub enum ValidationStatus {
    Success,
    NeedUpdate(Vec<(String, RemoteFile)>, Vec<PathBuf>),
}

fn resource_exists(resource: &str) -> bool {
    PathBuf::from(resource)
        .read_dir()
        .map(|mut dir| dir.next().is_some())
        .unwrap_or(false)
}

macro_rules! extend {
    ($files:expr, $resource:expr) => {
        $files.extend($resource);
    };
    ($files:expr, $resource:expr, $($resources:expr),+) => {
        extend!($files, $resource);
        extend!($files, $($resources),+);
    };
}

macro_rules! check_resources {
    ($file_server:expr, $handler:expr, $($resources:expr),+) => {
        let mut files = Vec::new();
        exists!(files, $($resources),+);
        downloader::download(files, $handler).await?;
    };
}

macro_rules! exists {
    ($files:expr, $resource:expr) => {
        if !resource_exists(&stringify!($resource)[10..]) {
            $files.extend($resource);
        }
    };
    ($files:expr, $resource:expr, $($resources:expr),+) => {
        exists!($files, $resource);
        exists!($files, $($resources),+);
    };
}

pub fn new_remote_directory(resources: ProfileResourcesResponse) -> RemoteDirectory {
    let mut files = RemoteDirectory::new();
    extend!(
        files,
        resources.profile.clone(),
        resources.libraries.clone(),
        resources.assets.clone(),
        resources.natives.clone(),
        resources.jre.clone()
    );
    files
}

pub fn create_hashed_file<P: AsRef<Path>>(path: P) -> Result<HashedFile> {
    let mut buffer = Vec::new();
    File::open(path)?.read_to_end(&mut buffer)?;
    Ok(HashedFile {
        size: buffer.len() as u64,
        checksum: t1ha::t1ha2_atonce128(buffer.as_slice(), 1),
    })
}

pub async fn validate_profile(
    profile: &Profile,
    files: &RemoteDirectory,
    handler: Handle<()>,
) -> Result<WatcherService> {
    let verify = &profile.update_verify;
    let exclude = &profile.update_exclusion;

    handler.dispatch(move |w| {
        w.eval("app.backend.download.wait()")?;
        Ok(())
    })?;
    match validate(&files, verify, exclude)? {
        ValidationStatus::NeedUpdate(files_to_update, file_to_remove) => {
            debug!("Files to download: {:?}", files_to_update);
            debug!("Files to remove: {:?}", file_to_remove);
            downloader::download(files_to_update, handler).await?;
            for path in file_to_remove {
                tokio::fs::remove_file(path).await?
            }
        }
        _ => {}
    }
    let watcher = WatcherService::new(profile).expect("Failed to create WatcherService");
    match validate(&files, verify, exclude)? {
        ValidationStatus::Success => Ok(watcher),
        ValidationStatus::NeedUpdate(files, file_to_remove) => Err(anyhow::anyhow!(
            "Sync error: {:?}",
            files
                .into_iter()
                .map(|file| file.0)
                .chain(
                    file_to_remove
                        .into_iter()
                        .map(|p| p.to_string_lossy().to_string())
                )
                .take(5)
                .collect::<Vec<_>>()
        )),
    }
}

fn validate(
    profile: &RemoteDirectory,
    verify: &[String],
    exclude: &[String],
) -> Result<ValidationStatus> {
    let mut remove_files = Vec::new();
    for dir in verify.iter().map(Path::new).filter(|path| path.is_dir()) {
        for file in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let file_path = file.path();
            if !profile.contains_key(file_path) {
                remove_files.push(file.into_path());
            }
        }
    }

    let profile = profile
        .iter()
        .filter(|&file| exclude.iter().all(|p| !file.0.starts_with(p)))
        .filter(|&file| {
            create_hashed_file(file.0).map_or(true, |ref hashed_file| hashed_file != file.1)
        });
    let profile = profile
        .map(|file| (file.0.to_slash_lossy(), file.1.clone()))
        .collect::<Vec<(String, RemoteFile)>>();
    if profile.is_empty() && remove_files.is_empty() {
        Ok(ValidationStatus::Success)
    } else {
        Ok(ValidationStatus::NeedUpdate(profile, remove_files))
    }
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
