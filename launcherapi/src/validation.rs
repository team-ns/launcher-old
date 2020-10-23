use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct HashedFile {
    pub size: u64,
    pub checksum: u128,
}

#[derive(Deserialize, Serialize)]
pub struct RemoteFile {
    pub name: String,
    pub size: u64,
}

impl HashedFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut buffer = Vec::new();
        File::open(path)?.read_to_end(&mut buffer)?;
        Ok(HashedFile {
            size: buffer.len() as u64,
            checksum: t1ha::t1ha2_atonce128(buffer.as_slice(), 1),
        })
    }
}

pub type HashedDirectory = HashMap<String, HashedFile>;

#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
pub enum OsType {
    LinuxX64,
    LinuxX32,
    MacOSX64,
    WindowsX64,
    WindowsX32,
}
