#[cfg(target_os = "windows")]
use std::path::Path;

use anyhow::Result;
use serde_json::Value;
use wry::application::dpi::PhysicalSize;
use wry::application::event_loop::{EventLoop, EventLoopProxy};
use wry::application::window::Window;
use wry::webview::WebView;

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

pub fn create_window() -> Result<(Window, EventLoop<WebviewEvent>)> {
    use wry::application::window::WindowBuilder;

    let event_loop = EventLoop::<WebviewEvent>::with_user_event();

    let window = WindowBuilder::new()
        .with_decorations(!BUNDLE.window.frameless)
        .with_title(&BUNDLE.project_name)
        .with_resizable(BUNDLE.window.resizable)
        .with_transparent(BUNDLE.window.transparent)
        .with_inner_size(PhysicalSize::new(BUNDLE.window.width, BUNDLE.window.height))
        .build(&event_loop)
        .unwrap();

    Ok((window, event_loop))
}

pub fn create_webview(window: Window, proxy: EventProxy) -> Result<WebView> {
    use wry::webview::WebViewBuilder;
    let webview = WebViewBuilder::new(window).unwrap();
    let webview = webview
        .with_initialization_script(&get_initialization_script())
        .with_custom_protocol("nslauncher".to_string(), move |_, _| {
            Ok((get_runtime(), "text/html".to_string()))
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
    Ok(webview)
}

#[cfg(feature = "bundle")]
fn get_runtime() -> Vec<u8> {
    include_crypt::include_crypt!(AES, "runtime/index.html").decrypt()
}

#[cfg(not(feature = "bundle"))]
fn get_runtime() -> Vec<u8> {
    use std::fs;
    fs::read("runtime/index.html").expect("Can't read lazy runtime file")
}

#[cfg(windows)]
pub fn download_webview2() {
    use std::fs;
    use std::io::Write;
    let installer = minreq::get("https://go.microsoft.com/fwlink/p/?LinkId=2124703")
        .send()
        .unwrap();
    let body = installer.as_bytes().to_vec();
    let temp_dir = temp_dir::TempDir::new().expect("Can't create temdir");
    let install_path = temp_dir.child("install.exe");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&install_path)
        .unwrap();
    file.write_all(&body).unwrap();
    drop(file);
    run_admin(&install_path, &["/silent", "/install"]).expect("Can't run installer");
}

#[cfg(target_os = "windows")]
fn run_admin<FP: AsRef<Path>, AP: AsRef<Path>>(file: FP, args: &[AP]) -> Result<()> {
    use std::ffi::OsStr;
    use std::mem;
    use std::os::windows::ffi::OsStrExt;

    let mut params = String::new();
    for arg in args.iter() {
        let arg = arg.as_ref().to_string_lossy();
        params.push(' ');
        if arg.len() == 0 {
            params.push_str("\"\"");
        } else if arg.find(&[' ', '\t', '"'][..]).is_none() {
            params.push_str(&arg);
        } else {
            params.push('"');
            for c in arg.chars() {
                match c {
                    '\\' => params.push_str("\\\\"),
                    '"' => params.push_str("\\\""),
                    c => params.push(c),
                }
            }
            params.push('"');
        }
    }

    let file = OsStr::new(file.as_ref())
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let params = OsStr::new(&params)
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();

    unsafe {
        use winapi::shared::minwindef::DWORD;
        use winapi::um::processthreadsapi::GetExitCodeProcess;
        use winapi::um::shellapi::{
            ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        };
        use winapi::um::synchapi::WaitForSingleObject;
        use winapi::um::winbase::INFINITE;
        use winapi::um::winuser::SW_NORMAL;

        let mut sei: SHELLEXECUTEINFOW = mem::zeroed();

        sei.fMask = SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS;
        sei.lpVerb = OsStr::new("runas")
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
            .as_ptr();
        sei.lpFile = file.as_ptr();
        sei.lpParameters = params.as_ptr();
        sei.nShow = SW_NORMAL;

        let result = ShellExecuteExW(&mut sei);
        if result == 0 || sei.hProcess.is_null() {
            return Err(anyhow::anyhow!("Can't execute command"));
        }

        WaitForSingleObject(sei.hProcess, INFINITE);

        let mut code: DWORD = mem::zeroed();
        let result = GetExitCodeProcess(sei.hProcess, &mut code);

        if result == 0 {
            return Err(anyhow::anyhow!("Can't get proccess exit code"));
        }
    }

    Ok(())
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
