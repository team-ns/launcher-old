use dlopen::symbor::{Library, Symbol};

use launcher_extension_api::command::{CommandRegister, ExtensionCommand};

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

pub struct ExtensionManager {
    extensions: HashMap<String, DynamicExtension>,
}

pub struct DynamicExtension {
    _lib: Library,
    pub extension: Box<dyn LauncherExtension>,
}

impl ExtensionManager {
    pub fn load_extensions() -> Self {
        fs::create_dir_all("extensions").expect("Can't create extension folder");
        let mut extensions = HashMap::new();
        for entry in crate::util::get_files_from_dir("extensions").filter(|e| {
            e.path()
                .extension()
                .map(|ex| ex.eq(FILE_EXTENSION))
                .unwrap_or(false)
        }) {
            let lib = Library::open(entry.path()).expect("Can't load library");
            let new_extension: Symbol<ExtensionFn> =
                unsafe { lib.symbol("new_extension") }.expect("Can't load symbol");
            let extension: (String, Box<dyn LauncherExtension>) = new_extension();
            let dynamic_extension = DynamicExtension {
                _lib: lib,
                extension: extension.1,
            };
            extensions.insert(extension.0, dynamic_extension);
        }
        ExtensionManager { extensions }
    }

    pub fn initialize_extensions(&self) -> Result<()> {
        for de in self.extensions.values() {
            de.extension.init()?
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
}
