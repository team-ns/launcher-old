use dlopen::symbor::{Library, Symbol};

use launcher_extension_api::command::{CommandRegister, ExtensionCommand};

use crate::util;
use launcher_extension_api::{LauncherExtension, Result};
use std::collections::HashMap;
use std::fs;

#[cfg(target_os = "linux")]
const FILE_EXTENSION: &str = "so";
#[cfg(target_os = "macos")]
const FILE_EXTENSION: &str = "dylib";
#[cfg(target_os = "windows")]
const FILE_EXTENSION: &str = "dll";

type ExtensionFn = fn() -> (String, Box<dyn LauncherExtension>);

pub struct ExtensionService {
    extensions: HashMap<String, ExtensionLibrary>,
}

pub struct ExtensionLibrary {
    _lib: Library,
    pub extension: Box<dyn LauncherExtension>,
}

#[teloc::inject]
impl ExtensionService {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ExtensionService {
    fn default() -> Self {
        Self::load_extensions()
    }
}

impl ExtensionService {
    pub fn load_extensions() -> Self {
        log::info!("Loading launchserver extensions");
        fs::create_dir_all("extensions").expect("Can't create extension folder");
        let mut extensions = HashMap::new();
        for entry in util::fs::get_files_from_dir("extensions").filter(|e| {
            e.path()
                .extension()
                .map(|ex| ex.eq(FILE_EXTENSION))
                .unwrap_or(false)
        }) {
            let lib = Library::open(entry.path()).expect("Can't load library");
            let new_extension: Symbol<ExtensionFn> =
                unsafe { lib.symbol("new_extension") }.expect("Can't load symbol");
            let extension: (String, Box<dyn LauncherExtension>) = new_extension();
            let dynamic_extension = ExtensionLibrary {
                _lib: lib,
                extension: extension.1,
            };
            extensions.insert(extension.0, dynamic_extension);
        }
        ExtensionService { extensions }
    }

    pub fn initialize_extensions(&self) -> Result<()> {
        for library in self.extensions.values() {
            library.extension.init()?
        }
        Ok(())
    }

    pub fn get_commands(&self) -> HashMap<String, HashMap<String, ExtensionCommand>> {
        let mut commands = HashMap::new();
        for (extension_name, dynamic_extension) in &self.extensions {
            let mut register = CommandRegister::default();
            dynamic_extension.extension.register_command(&mut register);
            commands.insert(extension_name.to_string(), register.into_commands());
        }
        commands
    }

    pub fn handle_auth(&self, login: &str, password: &str, ip: &str) -> Result<()> {
        for library in self.extensions.values() {
            library.extension.pre_auth(login, password, ip)?;
        }
        Ok(())
    }
}
