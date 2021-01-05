use anyhow::Result;
use launcher_api::message::ProfileResourcesResponse;
use launcher_api::validation::{HashedFile, OsType, RemoteDirectory, RemoteFile};
use std::path::{Path, PathBuf};
use web_view::Handle;

use crate::client::downloader;
use crate::security::watcher::WatcherService;
use launcher_api::profile::Profile;
use std::fs::File;
use std::io::Read;

pub enum ValidationStatus {
    Success,
    NeedUpdate(Vec<(String, RemoteFile)>),
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

pub async fn validate_profile(
    profile: &Profile,
    resources: ProfileResourcesResponse,
    handler: Handle<()>,
) -> Result<WatcherService> {
    let mut files = RemoteDirectory::new();
    extend!(
        files,
        resources.profile.clone(),
        resources.libraries.clone(),
        resources.assets.clone(),
        resources.natives.clone(),
        resources.jre.clone()
    );

    handler.dispatch(move |w| {
        w.eval("app.backend.download.wait()");
        Ok(())
    });
    match validate(&files)? {
        ValidationStatus::NeedUpdate(files_to_update) => {
            downloader::download(files_to_update, handler).await?;
        }
        _ => {}
    }
    let watcher = WatcherService::new(profile).expect("Failed to create WatcherService");
    match validate(&files)? {
        ValidationStatus::Success => Ok(watcher),
        ValidationStatus::NeedUpdate(files) => Err(anyhow::anyhow!(
            "Sync error: {:?}",
            files
                .into_iter()
                .take(5)
                .map(|file| file.0)
                .collect::<Vec<_>>()
        )),
    }
}

fn validate(profile: &RemoteDirectory) -> Result<ValidationStatus> {
    fn create_hashed_file<P: AsRef<Path>>(path: P) -> Result<HashedFile> {
        let mut buffer = Vec::new();
        File::open(path)?.read_to_end(&mut buffer)?;
        Ok(HashedFile {
            size: buffer.len() as u64,
            checksum: t1ha::t1ha2_atonce128(buffer.as_slice(), 1),
        })
    }

    let profile = profile.iter().filter(|&file| {
        create_hashed_file(file.0).map_or(true, |ref hashed_file| hashed_file != file.1)
    });
    let profile = profile
        .map(|file| (file.0.to_string(), file.1.clone()))
        .collect::<Vec<(String, RemoteFile)>>();
    if profile.is_empty() {
        Ok(ValidationStatus::Success)
    } else {
        Ok(ValidationStatus::NeedUpdate(profile))
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
