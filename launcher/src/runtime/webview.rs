use crate::config::CONFIG;
use crate::runtime::invoke_handler;
use crate::runtime::messages::RuntimeMessage;
use anyhow::Result;

use std::fs;
use std::io::Write;
use tokio::sync::mpsc::UnboundedSender;
use wry::application::dpi::PhysicalSize;
use wry::application::event_loop::{EventLoop, EventLoopProxy};
use wry::webview::WebView;

pub type EventProxy = EventLoopProxy<WebviewEvent>;

#[derive(Debug)]
pub enum WebviewEvent {
    DispatchScript(String),
    HideWindow,
    Exit,
}

pub fn create_webview(
    tx: UnboundedSender<(RuntimeMessage, EventProxy)>,
) -> Result<(WebView, EventLoop<WebviewEvent>)> {
    use wry::{application::window::WindowBuilder, webview::WebViewBuilder};

    let event_loop = new_event_loop();
    let window = WindowBuilder::new()
        .with_decorations(true)
        .with_title(&CONFIG.project_name)
        .with_resizable(false)
        .with_inner_size(PhysicalSize::new(1000, 600))
        .build(&event_loop)
        .unwrap();

    let proxy = event_loop.create_proxy();
    let webview = WebViewBuilder::new(window)
        .unwrap()
        .with_custom_protocol("nslauncher".to_string(), move |_, _| Ok(get_runtime()))
        .with_url("nslauncher://")?
        .with_rpc_handler(move |window, req| invoke_handler(window, req, tx.clone(), proxy.clone()))
        .build()?;
    Ok((webview, event_loop))
}

#[cfg(feature = "bundle")]
fn get_runtime() -> Vec<u8> {
    Ok(include_crypt!(AES, "runtime/index.html").decrypt())
}

#[cfg(not(feature = "bundle"))]
fn get_runtime() -> Vec<u8> {
    fs::read("runtime/index.html").expect("Can't read lazy runtime file")
}

#[cfg(windows)]
fn new_event_loop<T>() -> EventLoop<T> {
    use wry::application::platform::windows::EventLoopExtWindows;
    EventLoop::new_any_thread()
}

#[cfg(windows)]
pub fn download_webview2() {
    let installer = minreq::get("https://go.microsoft.com/fwlink/p/?LinkId=2124703")
        .send()
        .unwrap();
    let body = installer.as_bytes().to_vec();
    let temp_dir = temp_dir::TempDir::new().expect("Can't create temdir");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(temp_dir.child("install.exe"))
        .unwrap();
    file.write_all(&body).unwrap();
    drop(file);
    runas::Command::new("install.exe")
        .args(&["/silent", "/install"])
        .status()
        .expect("Can't run installer");
}

#[cfg(unix)]
fn new_event_loop<T>() -> EventLoop<T> {
    use wry::application::platform::unix::EventLoopExtUnix;
    EventLoop::new_any_thread()
}
