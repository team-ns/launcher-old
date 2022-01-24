use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

use crate::client::Client;
use crate::runtime::arg::{Invoke, InvokeMessage, InvokePayload, InvokeResolver};
use crate::runtime::webview::EventProxy;

mod arg;
mod messages;
mod rpc;
pub mod webview;

pub static CLIENT: OnceCell<Arc<Mutex<Client>>> = OnceCell::new();

pub static PLAYING: OnceCell<()> = OnceCell::new();

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

pub fn run() {
    if cfg!(target_os = "windows") && !webview::has_webview() {
        webview::install_webview2()
    }
    webview::launch().expect("Can't run launcher webview");
}

fn handle(proxy: EventProxy, command: String, payload: InvokePayload) -> Result<()> {
    let message = InvokeMessage {
        proxy: proxy.clone(),
        command,
        payload: payload.inner,
    };
    let resolver = InvokeResolver {
        proxy,
        callback: payload.callback,
        error: payload.error,
    };
    let invoke = Invoke { message, resolver };
    crate::runtime::messages::handle(invoke);
    Ok(())
}

pub fn spawn<F>(task: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let runtime = RUNTIME.get_or_init(|| Runtime::new().unwrap());
    runtime.spawn(task);
}
