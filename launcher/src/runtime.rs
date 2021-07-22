use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::ControlFlow;

use crate::client::Client;
use crate::runtime::arg::{Invoke, InvokeMessage, InvokePayload, InvokeResolver};
use crate::runtime::webview::{create_webview, create_window, EventProxy, WebviewEvent};

mod arg;
mod messages;
mod rpc;
pub mod webview;

pub static CLIENT: OnceCell<Arc<Mutex<Client>>> = OnceCell::new();

pub static PLAYING: OnceCell<()> = OnceCell::new();

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

pub fn run() {
    let (window, event_loop) = create_window().expect("Can't create window");
    let proxy = event_loop.create_proxy();
    let (webview, event_loop) = match create_webview(window, proxy) {
        Ok(w) => (w, event_loop),
        Err(e) => {
            #[cfg(target_os = "windows")]
            {
                use wry::Error as WVError;
                match e.downcast::<WVError>() {
                    Err(e) => {
                        panic!("{}", e)
                    }
                    Ok(WVError::WebView2Error(e)) => {
                        if e.hresult() == -2147024894 {
                            webview::download_webview2();
                            drop(event_loop);
                            let (window, event_loop) =
                                create_window().expect("Can't create window");
                            let proxy = event_loop.create_proxy();
                            let w = create_webview(window, proxy).expect("Can't create webview");
                            (w, event_loop)
                        } else {
                            panic!("{:?}", e)
                        }
                    }
                    Ok(e) => panic!("{:?}", e),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                panic!("{}", e)
            }
        }
    };
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
