use serde::Deserialize;

use crate::runtime::arg::{Invoke, InvokeMessage, InvokeResolver};
use crate::runtime::webview::EventProxy;

mod event;
mod launcher;
mod library;
mod storage;
mod system;
mod window;

#[derive(Deserialize)]
#[serde(tag = "module", content = "message")]
enum Module {
    Window(window::Cmd),
    Library(library::Cmd),
    System(system::Cmd),
    Storage(storage::Cmd),
    Launcher(launcher::Cmd),
    Event(event::Cmd),
}

impl Module {
    fn run(self, resolver: InvokeResolver, proxy: EventProxy) {
        match self {
            Self::Launcher(cmd) => crate::runtime::spawn(async { cmd.run(resolver, proxy).await }),
            Self::Storage(cmd) => crate::runtime::spawn(async { cmd.run(resolver).await }),
            Self::System(cmd) => cmd.run(resolver),
            Self::Window(cmd) => cmd.run(resolver, proxy),
            Self::Library(cmd) => cmd.run(resolver),
            Self::Event(cmd) => cmd.run(resolver, proxy),
        };
    }
}

pub fn handle(invoke: Invoke) {
    let Invoke { message, resolver } = invoke;
    let InvokeMessage { payload, proxy, .. } = message;

    match serde_json::from_value::<Module>(payload) {
        Ok(module) => module.run(resolver, proxy),
        Err(e) => resolver.reject(e.to_string()),
    };
}
