use crate::optional::OptionalFiles;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Clone)]
pub struct HashedFile {
    pub size: u64,
    pub checksum: u128,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct RemoteFile {
    pub uri: String,
    pub size: u64,
    pub checksum: u128,
}

impl PartialEq<RemoteFile> for HashedFile {
    fn eq(&self, other: &RemoteFile) -> bool {
        self.size == other.size && self.checksum == other.checksum
    }
}

pub type RemoteDirectory = HashMap<PathBuf, RemoteFile>;

pub trait RemoteDirectoryExt {
    fn filter_files(self, files: Option<&OptionalFiles>) -> Self;
}

impl RemoteDirectoryExt for RemoteDirectory {
    fn filter_files(mut self, files: Option<&OptionalFiles>) -> Self {
        if let Some(files) = files {
            for path in &files.original_paths {
                self.remove(path);
            }
            for path in &files.rename_paths {
                if let Some(file) = self.remove(path.0) {
                    self.insert(PathBuf::from(path.1), file);
                }
            }
        }
        self
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Clone)]
pub enum OsType {
    LinuxX64,
    LinuxX32,
    MacOsX64,
    WindowsX64,
    WindowsX32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ClientInfo {
    pub os_type: OsType,
}
