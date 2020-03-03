use std::fmt;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::io::Error as IoError;
use self_encryption::{StorageError, Storage};
use std::path::PathBuf;
use futures::{Future, future};
use std::fs::File;
use std::io::{Read, Write};
use tiny_keccak::{Sha3, Hasher};

#[derive(Debug)]
pub struct DiskStorageError {
    io_error: IoError,
}

impl Display for DiskStorageError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "I/O error getting/putting: {}", self.io_error)
    }
}

impl StdError for DiskStorageError {
    fn description(&self) -> &str {
        "DiskBasedStorage Error"
    }
}

impl From<IoError> for DiskStorageError {
    fn from(error: IoError) -> DiskStorageError {
        DiskStorageError { io_error: error }
    }
}

impl StorageError for DiskStorageError {}

pub struct DiskStorage {
    pub storage_path: String,
}

fn file_name(name: &[u8]) -> String {
    let mut string = String::new();
    for ch in name {
        string.push_str(&fmt::format(format_args!("{:02x}", *ch)));
    }
    string
}

impl DiskStorage {
    pub fn new(path: &str) -> DiskStorage {
        DiskStorage { storage_path: path.to_string() }
    }

    fn calculate_path(&self, name: &[u8]) -> PathBuf {
        let mut path = PathBuf::from(self.storage_path.clone());
        path.push(file_name(name));
        path
    }
}

impl Storage for DiskStorage {
    type Error = DiskStorageError;

    fn get(&self, name: &[u8]) -> Box<dyn Future<Item = Vec<u8>, Error =DiskStorageError>> {
        let path = self.calculate_path(name);
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(error) => return Box::new(future::err(From::from(error))),
        };
        let mut data = Vec::new();
        let result = file
            .read_to_end(&mut data)
            .map(move |_| data)
            .map_err(From::from);
        Box::new(future::result(result))
    }

    fn put(
        &mut self,
        name: Vec<u8>,
        data: Vec<u8>,
    ) -> Box<dyn Future<Item = (), Error =DiskStorageError>> {
        let path = self.calculate_path(&name);
        let mut file = match File::create(&path) {
            Ok(file) => file,
            Err(error) => return Box::new(future::err(From::from(error))),
        };

        let result = file
            .write_all(&data[..])
            .map(|_| {
                println!("Chunk written to {:?}", path);
            })
            .map_err(From::from);
        Box::new(future::result(result))
    }

    fn generate_address(&self, data: &[u8]) -> Vec<u8> {
        let mut sha3 = Sha3::v256();
        let mut data = data.to_vec();
        sha3.update(&data);
        sha3.finalize(&mut data);
        data
    }
}