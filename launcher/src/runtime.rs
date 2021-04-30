use crate::client::Client;

use log::{debug, error};
use messages::RuntimeMessage;
use once_cell::sync::OnceCell;
use std::sync::Arc;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;

use crate::config::SETTINGS;
use crate::runtime::webview::{create_webview, EventProxy, WebviewEvent};
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::ControlFlow;
use wry::application::window::Window;
use wry::webview::{RpcRequest, RpcResponse};
use wry::{Error as WVError, Value};

mod messages;
pub mod webview;

pub static CLIENT: OnceCell<Arc<Mutex<Client>>> = OnceCell::new();

pub static PLAYING: OnceCell<()> = OnceCell::new();

#[macro_export]
macro_rules! handle_error {
    ($handler:expr, $result:expr) => {
        if let Err(error) = $result {
            error!("Runtime message error: {}", error);
            $handler
                .send_event(WebviewEvent::DispatchScript(format!(
                    r#"app.backend.error("{}")"#,
                    error.to_string().replace(r#"""#, r#"""#)
                )))
                .expect("Can't eval error");
        }
    };
}

pub async fn start() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let (runtime_message_tx, mut runtime_message_rx) =
        tokio::sync::mpsc::unbounded_channel::<String>();
    let ui_handle = tokio::task::spawn_blocking(move || {
        let (mut webview, event_loop) = match create_webview(tx.clone()) {
            Ok(w) => w,
            Err(e) => {
                if cfg!(windows) {
                    match e.downcast::<WVError>() {
                        Err(e) => {
                            panic!("{}", e)
                        }
                        Ok(WVError::WebView2Error(e)) => {
                            if e.hresult() == -2147024894 {
                                webview::download_webview2();
                                create_webview(tx.clone()).expect("Can't create webview")
                            } else {
                                panic!("{:?}", e)
                            }
                        }
                        Ok(e) => panic!("{:?}", e),
                    }
                } else {
                    panic!("{}", e)
                }
            }
        };
        let dispatcher = webview.dispatcher();
        tokio::spawn(async move {
            loop {
                match runtime_message_rx.recv().await {
                    None => break,
                    Some(message) => {
                        dispatcher
                            .dispatch_script(&format!(
                                r#"app.backend.customMessage('{}')"#,
                                message.replace(r#"""#, r#"""#)
                            ))
                            .expect("Can't eval message request");
                    }
                }
            }
        });
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                Event::UserEvent(event) => match event {
                    WebviewEvent::DispatchScript(s) => {
                        webview.dispatch_script(&s).expect("Can't dispatch js");
                        webview.evaluate_script().expect("Can't invoke js")
                    }
                    WebviewEvent::HideWindow => webview.window().set_visible(false),
                    WebviewEvent::Exit => {
                        *control_flow = ControlFlow::Exit;
                    }
                },
                _ => {
                    let _ = webview.resize();
                }
            }
        });
    });
    let message_handle = tokio::task::spawn(async move {
        message_loop(rx, runtime_message_tx).await;
    });
    ui_handle.await.expect("Can't execute ui loop");
    if PLAYING.get().is_none() {
        std::process::exit(0);
    }
    message_handle.await.expect("Can't execute message loop");
}

fn invoke_handler(
    _window: &Window,
    mut req: RpcRequest,
    sender: UnboundedSender<(RuntimeMessage, EventProxy)>,
    dispatcher: EventProxy,
) -> Option<RpcResponse> {
    debug!("Request rpc: {:?}", req);
    if &req.method == "launcher" {
        if let Some(mut params) = req.params.take() {
            if params.is_array() {
                let msg = params[0].take();
                debug!("{}", msg);
                if let Ok(msg) = serde_json::from_value::<RuntimeMessage>(msg) {
                    debug!("Runtime message: {:?}", msg);
                    sender
                        .send((msg, dispatcher))
                        .unwrap_or_else(|_| panic!("Can't send message to logic loop"));
                }
            }
        }
    } else if &req.method == "settings::setProperty" {
        if let Some(Value::Array(mut args)) = req.params.take() {
            if !args.is_empty() && args.len() > 1 {
                let name = args.swap_remove(0);
                let value = args.swap_remove(0);
                let mut settings = match SETTINGS.get().expect("Can't get settings").try_lock() {
                    Ok(s) => s,
                    Err(_) => {
                        return Some(RpcResponse::new_error(
                            req.id,
                            Some(serde_json::json!("Can't save property")),
                        ));
                    }
                };
                settings.properties.insert(
                    name.as_str().unwrap_or("null").to_string(),
                    serde_json::to_string(&value).expect("Can't save value"),
                );
                settings.save().expect("Can't save settings")
            };
        }
    } else if &req.method == "settings::getProperty" {
        if let Some(Value::Array(mut args)) = req.params.take() {
            if !args.is_empty() {
                let name = args.swap_remove(0);
                let settings = match SETTINGS.get().expect("Can't get settings").try_lock() {
                    Ok(s) => s,
                    Err(_) => {
                        return Some(RpcResponse::new_error(
                            req.id,
                            Some(serde_json::json!("Can't get property")),
                        ));
                    }
                };
                let property = settings
                    .properties
                    .get(name.as_str().unwrap_or("null"))
                    .cloned()
                    .map(|v| serde_json::from_str::<Value>(&v).ok())
                    .flatten();
                return Some(RpcResponse::new_result(req.id, property));
            };
        }
    } else if &req.method == "settings::removeProperty" {
        if let Some(Value::Array(mut args)) = req.params.take() {
            if !args.is_empty() {
                let name = args.swap_remove(0);
                let mut settings = match SETTINGS.get().expect("Can't get settings").try_lock() {
                    Ok(s) => s,
                    Err(_) => {
                        return Some(RpcResponse::new_error(
                            req.id,
                            Some(serde_json::json!("Can't remove property")),
                        ));
                    }
                };
                settings.properties.remove(name.as_str().unwrap_or("null"));
                settings.save().expect("Can't save settings");
            };
        }
    }
    None
}

async fn message_loop(
    mut recv: UnboundedReceiver<(RuntimeMessage, EventProxy)>,
    sender: UnboundedSender<String>,
) {
    loop {
        match recv.recv().await {
            None => {
                break;
            }
            Some(message) => {
                let handler = message.1;
                let error_handler = handler.clone();
                let message = message.0;
                match message {
                    RuntimeMessage::Login {
                        login,
                        password,
                        remember_me,
                    } => {
                        let client = Arc::clone(CLIENT.get().expect("Client not found"));
                        handle_error!(
                            error_handler,
                            messages::login(login, password, remember_me, client, handler).await
                        );
                    }
                    RuntimeMessage::Play { profile } => {
                        let client = Arc::clone(CLIENT.get().expect("Client not found"));
                        handle_error!(
                            error_handler,
                            messages::start_client(handler, client, profile).await
                        )
                    }
                    RuntimeMessage::Ready => {
                        handle_error!(
                            error_handler,
                            messages::ready(handler, sender.clone()).await
                        )
                    }
                    RuntimeMessage::SelectGameDir => {
                        handle_error!(error_handler, messages::select_game_dir(handler).await)
                    }
                    RuntimeMessage::SaveSettings(settings) => {
                        handle_error!(
                            error_handler,
                            messages::save_settings(settings, handler).await
                        )
                    }
                    RuntimeMessage::Logout => {
                        let client = Arc::clone(CLIENT.get().expect("Client not found"));
                        handle_error!(error_handler, messages::logout(client).await)
                    }
                };
            }
        };
    }
}

#[cfg(test)]
mod test {
    use crate::runtime::messages::RuntimeMessage;

    #[test]
    fn test_value() {
        let value = serde_json::to_value("ready").unwrap();
        let message = serde_json::from_value::<RuntimeMessage>(value).unwrap();
        assert_eq!(message, RuntimeMessage::Ready);
    }
}
