use anyhow::Result;
use std::ffi::OsStr;
use std::mem;
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;
use wry::application::dpi::PhysicalSize;
use wry::application::event_loop::{EventLoop, EventLoopProxy};
use wry::webview::WebView;

use crate::config::BUNDLE;
use crate::runtime::invoke_handler;
use crate::runtime::messages::RuntimeMessage;

pub type EventProxy = EventLoopProxy<WebviewEvent>;

#[derive(Debug, Clone)]
pub enum WebviewEvent {
    DispatchScript(String),
    HideWindow,
    Exit,
}

pub fn create_webview(
    tx: UnboundedSender<(RuntimeMessage, EventProxy)>,
) -> Result<(WebView, EventLoop<WebviewEvent>)> {
    use wry::{application::window::WindowBuilder, webview::WebViewBuilder};

    let event_loop = create_event_loop();
    let window = WindowBuilder::new()
        .with_decorations(!BUNDLE.window.frameless)
        .with_title(&BUNDLE.project_name)
        .with_resizable(BUNDLE.window.resizable)
        .with_transparent(BUNDLE.window.transparent)
        .with_inner_size(PhysicalSize::new(BUNDLE.window.width, BUNDLE.window.height))
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

fn create_event_loop() -> EventLoop<WebviewEvent> {
    #[cfg(target_os = "linux")]
    {
        EventLoop::<WebviewEvent>::new_any_thread()
    }
    #[cfg(target_os = "windows")]
    {
        use wry::application::platform::windows::EventLoopExtWindows;
        EventLoop::<WebviewEvent>::new_any_thread()
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
    {
        EventLoop::<WebviewEvent>::with_user_event()
    }
}
