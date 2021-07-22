use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use dlopen::symbor::{Library as DynamicLibrary, Symbol};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::runtime::arg::InvokeResolver;

type LibraryId = u32;
type LibraryStore = Arc<Mutex<HashMap<LibraryId, Library>>>;
type LibraryFn = extern "C" fn(*const c_char) -> *const c_char;

static LIBRARIES: Lazy<LibraryStore> = Lazy::new(Default::default);

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
    GetLibrary {
        library: String,
    },
    #[serde(rename_all = "camelCase")]
    Invoke {
        library: u32,
        symbol_name: String,
        payload: Value,
    },
}

impl Cmd {
    pub fn run(self, resolver: InvokeResolver) {
        match self {
            Cmd::GetLibrary { library } => resolver.resolve_result(get_library_id(&library)),
            Cmd::Invoke {
                library,
                symbol_name,
                payload,
            } => resolver.resolve_result(invoke_library(library, symbol_name, payload)),
        }
    }
}

struct Library {
    name: String,
    dynamic_library: DynamicLibrary,
}

fn get_library_id(library: &str) -> Result<LibraryId> {
    let libraries = LIBRARIES
        .lock()
        .map_err(|e| anyhow::anyhow!("Can't get library storage: {}", e))?;
    let library = libraries
        .iter()
        .find(|(_, lib)| lib.name.eq(library))
        .context("Library not found")?;
    Ok(*library.0)
}

fn invoke_library(library: u32, symbol_name: String, payload: Value) -> Result<Value> {
    let libraries = LIBRARIES
        .lock()
        .map_err(|e| anyhow::anyhow!("Can't get library storage: {}", e))?;
    let library = libraries.get(&library).context("Library not found")?;
    let function: Symbol<LibraryFn> = unsafe { library.dynamic_library.symbol(&symbol_name)? };
    let payload_cstr = CString::new(serde_json::to_string(&payload)?)?;
    let result_string = unsafe {
        CStr::from_ptr(function(payload_cstr.as_ptr()))
            .to_string_lossy()
            .into_owned()
    };
    let result = serde_json::from_str::<Value>(&result_string)?;
    Ok(result)
}
