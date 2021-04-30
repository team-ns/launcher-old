use anyhow::Result;
use launcher_api::validation::OsType;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, SeekFrom};
use tokio_byteorder::{AsyncReadBytesExt, LittleEndian};

pub async fn get_os_type<P: AsRef<Path> + Debug>(path: P) -> Result<OsType> {
    let extension = path.as_ref().extension().and_then(OsStr::to_str);
    if let Some(extension) = extension {
        if extension.eq("dll") {
            let mut file = File::open(path.as_ref()).await?;
            file.seek(SeekFrom::Start(0x3C)).await?;
            let pe_header = file.read_u32::<LittleEndian>().await?;
            file.seek(SeekFrom::Start((pe_header + 4u32) as u64))
                .await?;
            match file.read_u16::<LittleEndian>().await? {
                0x014c => Ok(OsType::WindowsX32),
                0x8664 => Ok(OsType::WindowsX64),
                _ => Err(anyhow::anyhow!(
                    "Failed to get archetype from PE header of file: {:?}!",
                    path
                )),
            }
        } else if extension.eq("so") {
            let mut file = File::open(path.as_ref()).await?;
            file.seek(SeekFrom::Start(4)).await?;
            match file.read_u8().await? {
                1 => Ok(OsType::LinuxX32),
                2 => Ok(OsType::LinuxX64),
                _ => Err(anyhow::anyhow!(
                    "Failed to get archetype from ELF header of file: {:?}!",
                    path.as_ref()
                )),
            }
        } else if extension.eq("dylib") || extension.eq("jnilib") {
            Ok(OsType::MacOsX64)
        } else {
            Err(anyhow::anyhow!(
                "Found excess file: {:?} in native dir!",
                path
            ))
        }
    } else {
        Err(anyhow::anyhow!("Cannot get file: {:?} extension!", path))
    }
}
