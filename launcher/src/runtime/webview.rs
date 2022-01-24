use anyhow::Result;
use serde_json::Value;
use wry::application::dpi::PhysicalSize;
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use wry::http::ResponseBuilder;

use crate::config::BUNDLE;
use crate::runtime::arg::InvokePayload;
use crate::runtime::handle;

pub type EventProxy = EventLoopProxy<WebviewEvent>;

#[derive(Debug, Clone)]
pub enum WebviewEvent {
    DispatchScript(String),
    HideWindow,
    ShowWindow,
    Emit(String, Value),
    Exit,
}

pub fn launch() -> Result<()> {
    use wry::application::window::WindowBuilder;
    use wry::webview::WebViewBuilder;

    let event_loop = EventLoop::<WebviewEvent>::with_user_event();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_decorations(!BUNDLE.window.frameless)
        .with_title(&BUNDLE.project_name)
        .with_resizable(BUNDLE.window.resizable)
        .with_transparent(BUNDLE.window.transparent)
        .with_inner_size(PhysicalSize::new(BUNDLE.window.width, BUNDLE.window.height))
        .build(&event_loop)
        .unwrap();

    let webview = WebViewBuilder::new(window).unwrap();
    let webview = webview
        .with_initialization_script(&get_initialization_script())
        .with_custom_protocol("nslauncher".to_string(), move |request| {
            let path = request
                .uri()
                .replace("nslauncher://", "")
                .replacen("/", "", 1);

            let content = get_runtime(&path);

            let (data, meta) = if let Some(data) = content {
                let mime_type = mime_guess::from_path(&path)
                    .first_or_text_plain()
                    .to_string();
                (data, mime_type)
            } else {
                (get_runtime("index.html").unwrap(), "text/html".to_string())
            };

            ResponseBuilder::new().mimetype(&meta).body(data)
        })
        .with_url("nslauncher://")?
        .with_rpc_handler(move |_window, request| {
            let command = request.method.clone();

            let arg = request
                .params
                .unwrap()
                .as_array_mut()
                .unwrap()
                .first_mut()
                .unwrap_or(&mut Value::Null)
                .take();
            match serde_json::from_value::<InvokePayload>(arg) {
                Ok(message) => {
                    let proxy = proxy.clone();
                    let _ = handle(proxy, command, message);
                }
                Err(error) => {
                    let proxy = proxy.clone();
                    let _ = proxy.send_event(WebviewEvent::DispatchScript(format!(
                        r#"console.error({})"#,
                        Value::String(error.to_string())
                    )));
                }
            }
            None
        })
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(event) => match event {
                WebviewEvent::DispatchScript(s) => {
                    webview.evaluate_script(&s).expect("Can't invoke js")
                }
                WebviewEvent::HideWindow => webview.window().set_visible(false),
                WebviewEvent::Exit => {
                    *control_flow = ControlFlow::Exit;
                }
                WebviewEvent::ShowWindow => webview.window().set_visible(true),
                WebviewEvent::Emit(event, payload) => webview
                    .evaluate_script(&format!(
                        "window['{}']({{event: {}, payload: {}}})",
                        "b03ebfab-8145-44ac-a4b6-d800ddfa3bba",
                        serde_json::to_string(&event).expect("Can't serialize event name"),
                        payload
                    ))
                    .expect("Can't emit event"),
            },
            _ => {
                let _ = webview.resize();
            }
        }
    });
}

#[cfg(feature = "bundle")]
fn get_runtime(path: &str) -> Option<Vec<u8>> {
    let folder = include_crypt::include_dir!(AES, "runtime");
    let file = folder.get(path);
    file.map(EncryptedFile::decrypt)
}

#[cfg(not(feature = "bundle"))]
fn get_runtime(path: &str) -> Option<Vec<u8>> {
    use std::fs;
    fs::canonicalize(&path).and_then(fs::read).ok()
}

#[cfg(windows)]
pub fn has_webview() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    if hkcu
        .open_subkey(
            "Software\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        )
        .is_ok()
    {
        return true;
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    if cfg!(target_arch = "x86") {
        if hklm
            .open_subkey(
                "SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
            )
            .is_ok()
        {
            return true;
        }
    } else if cfg!(target_arch = "x86_64") && hklm.open_subkey(
        "SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    ).is_ok() {
        return true;
    }

    false
}

#[cfg(windows)]
pub fn install_webview2() {
    use std::fs;
    use std::io::Write;
    use std::process;

    let installer = minreq::get("https://go.microsoft.com/fwlink/p/?LinkId=2124703")
        .send()
        .unwrap();
    let body = installer.as_bytes().to_vec();
    let temp_dir = temp_dir::TempDir::new().expect("Can't create temdir");
    let install_path = temp_dir.child("install.exe");
    {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&install_path)
            .unwrap();
        file.write_all(&body).unwrap();
    }
    process::Command::new(install_path)
        .args(&["/silent", "/install"])
        .status()
        .expect("Can't run installer");
}

fn get_initialization_script() -> String {
    return format!(
        "
      window['{queue}'] = [];
      window['{function}'] = function (eventData, ignoreQueue) {{
      const listeners = (window['{listeners}'] && window['{listeners}'][eventData.event]) || []
      if (!ignoreQueue && listeners.length === 0) {{
        window['{queue}'].push({{
          eventData: eventData
        }})
      }}
      if (listeners.length > 0) {{
           for (let i = listeners.length - 1; i >= 0; i--) {{
            const listener = listeners[i]
            eventData.id = listener.id
            listener.handler(eventData.payload)
        }}
      }}
    }}
    ",
        function = "b03ebfab-8145-44ac-a4b6-d800ddfa3bba",
        queue = "53a38c36-7040-4b88-aca2-b7390bbd3fd6",
        listeners = "08b8adb2-276a-49e0-b5d1-67ee5c381946"
    );
}
