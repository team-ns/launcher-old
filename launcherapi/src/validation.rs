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

#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
pub enum OsType {
    LinuxX64,
    LinuxX32,
    MacOSX64,
    WindowsX64,
    WindowsX32,
}
