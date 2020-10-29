use anyhow::Result;
use log::info;
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;

#[derive(RustEmbed)]
#[folder = "../launcher"]
struct Launcher;

#[derive(RustEmbed)]
#[folder = "../launcherapi"]
struct LauncherApi;

pub fn unpack_launcher() {
    unpack::<LauncherApi>("launcher").expect("Can't unpack launcher");
    unpack::<Launcher>("launcherapi").expect("Can't unpack api");
}

fn unpack<T: RustEmbed>(folder: &str) -> Result<()> {
    let path = Path::new(folder);
    if !path.is_dir() {
        info!("Unpack {}...", folder);

        for filename in T::iter() {
            let file_path = path.join(filename.as_ref());
            if let Some(parent) = file_path.parent() {
                create_dir_all(parent)?;
            }
            let mut file =
                File::create(file_path).expect(&format!("Can't create file {}", filename));
            if let Some(bytes) = T::get(filename.as_ref()) {
                file.write_all(&bytes)?;
            }
        }
    }
    Ok(())
}
