use log::info;
use std::path::Path;
use rust_embed::RustEmbed;
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::borrow::Cow;


#[derive(RustEmbed)]
#[folder = "../launcher"]
struct Launcher;

#[derive(RustEmbed)]
#[folder = "../launcherapi"]
struct LauncherApi;


pub fn unpack_launcher() {
    if !Path::new("launcher").is_dir() {
        info!("Unpack launcher...");
        unpack_client();
    }
    if !Path::new("launcherapi").is_dir() {
        info!("Unpack launcherapi...");
        unpack_api();
    }
}

fn unpack_client() {
    let path = Path::new("launcher");
    for filename in Launcher::iter() {
        let file_path = path.join(filename.as_ref());
        if let Some(parent) = file_path.parent() {
            create_dir_all(parent);
        }
        let mut file = File::create(file_path)
            .expect(&format!("Can't create file {}", filename));
        if let Some(bytes) = Launcher::get(filename.as_ref()) {
            file.write_all(&bytes);
        }
    }
}

fn unpack_api() {
    let path = Path::new("launcherapi");
    for filename in LauncherApi::iter() {
        let file_path = path.join(filename.as_ref());
        if let Some(parent) = file_path.parent() {
            create_dir_all(parent);
        }
        let mut file = File::create(file_path)
            .expect(&format!("Can't create file {}", filename));
        if let Some(bytes) = LauncherApi::get(filename.as_ref()) {
            file.write_all(&bytes);
        }
    }
}