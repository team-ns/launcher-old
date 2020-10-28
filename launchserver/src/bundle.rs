use log::info;
use std::path::Path;
use rust_embed::RustEmbed;
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::borrow::Cow;
use anyhow::Result;

#[derive(RustEmbed)]
#[folder = "../launcher"]
struct Launcher;

#[derive(RustEmbed)]
#[folder = "../launcherapi"]
struct LauncherApi;


pub fn unpack_launcher() {
    unpack::<LauncherApi>("launcher")
        .expect("Can't unpack launcher");
    unpack::<Launcher>("launcherapi")
        .expect("Can't unpack api");;
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
            let mut file = File::create(file_path)
                .expect(&format!("Can't create file {}", filename));
            if let Some(bytes) = T::get(filename.as_ref()) {
                file.write_all(&bytes)?;
            }
        }
    }
    Ok(())
}