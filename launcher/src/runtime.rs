use crate::client::Client;

use log::debug;
use messages::RuntimeMessage;
use once_cell::sync::OnceCell;
use std::sync::Arc;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;

use crate::config::CONFIG;
use std::{env, fs};
use web_view::{Content, Handle, WVResult, WebView};

mod messages;

pub static CLIENT: OnceCell<Arc<Mutex<Client>>> = OnceCell::new();

#[macro_export]
macro_rules! handle_error {
    ($handler:expr, $result:expr) => {
        if let Err(error) = $result {
            $handler.dispatch(move |w| {
                w.eval(&format!(
                    r#"app.backend.error("{}")"#,
                    error.to_string().replace(r#"""#, r#"""#)
                ));
                Ok(())
            });
        }
    };
}

pub async fn start() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let message_handle = tokio::task::spawn(async move {
        message_loop(rx).await;
    });
    fs::create_dir_all(&CONFIG.game_dir);
    env::set_current_dir(&CONFIG.game_dir);
    let ui_handle = tokio::task::spawn_blocking(move || {
        let webview = web_view::builder()
            .title("NSLauncher")
            .content(Content::Html(include_str!("../runtime/index.html")))
            .size(1000, 600)
            .resizable(false)
            .debug(true)
            .user_data(())
            .invoke_handler(move |view, arg| invoke_handler(view, arg, tx.clone()))
            .build()
            .expect("Can't create webview runtime");
        webview.run().expect("Can't run webview runtime");
    });
    ui_handle.await;
    drop(message_handle);
}

fn invoke_handler(
    view: &mut WebView<()>,
    arg: &str,
    sender: UnboundedSender<(RuntimeMessage, Handle<()>)>,
) -> WVResult<()> {
    let handler = view.handle();
    debug!("Argument from runtime: {}", arg);
    let message: RuntimeMessage =
        serde_json::from_str(arg).expect("Can't parse message from runtime");
    sender.send((message, handler));
    Ok(())
}

async fn message_loop(mut recv: UnboundedReceiver<(RuntimeMessage, Handle<()>)>) {
    loop {
        match recv.recv().await {
            None => {}
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
                        handle_error!(error_handler, messages::ready(handler).await)
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
