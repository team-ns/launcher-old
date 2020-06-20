use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct HashedFile {
    pub size: u64,
    pub checksum: u128,
}

impl HashedFile {
    pub fn new(path: &str) -> Self {
        //TODO add Error handling
        let mut buffer = Vec::new();
        File::open(path).unwrap().read_to_end(&mut buffer);
        HashedFile{ size: buffer.len() as u64, checksum: t1ha::t1ha2_atonce128(buffer.as_slice(), 1) }
    }
}

pub type HashedProfile = HashMap<String, HashedFile>;


