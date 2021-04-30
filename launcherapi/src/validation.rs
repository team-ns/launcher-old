use crate::optional::OptionalFiles;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Clone)]
pub struct HashedFile {
    pub size: usize,
    pub checksum: u128,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct RemoteFile {
    pub uri: String,
    pub size: usize,
    pub checksum: u128,
}

impl PartialEq<RemoteFile> for HashedFile {
    fn eq(&self, other: &RemoteFile) -> bool {
        self.size == other.size && self.checksum == other.checksum
    }
}

pub type RemoteDirectory = HashMap<PathBuf, RemoteFile>;

pub trait RemoteDirectoryExt {
    fn filter_files(
        self,
        files: (Option<&Vec<OptionalFiles>>, Option<&Vec<OptionalFiles>>),
    ) -> Self;
}

impl RemoteDirectoryExt for RemoteDirectory {
    fn filter_files(
        mut self,
        files: (Option<&Vec<OptionalFiles>>, Option<&Vec<OptionalFiles>>),
    ) -> Self {
        if let Some(irrelevant_files) = files.0 {
            for optional_files in irrelevant_files {
                for path in &optional_files.original_paths {
                    self.remove(path);
                }
                for path in &optional_files.rename_paths {
                    self.remove(path.0);
                }
            }
        }
        if let Some(relevant_files) = files.0 {
            for optional_files in relevant_files {
                for path in &optional_files.rename_paths {
                    if let Some(file) = self.remove(path.0) {
                        self.insert(PathBuf::from(path.1), file);
                    }
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

impl fmt::Display for OsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ClientInfo {
    pub os_type: OsType,
}
