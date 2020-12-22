use crate::client::Client;

use log::debug;
use messages::RuntimeMessage;
use once_cell::sync::OnceCell;
use std::sync::Arc;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use web_view::{Content, Handle, WVResult, WebView};

mod messages;

pub static CLIENT: OnceCell<Arc<Mutex<Client>>> = OnceCell::new();

pub async fn start() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let message_handle = tokio::task::spawn(async move {
        message_loop(rx).await;
    });
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
            .unwrap();
        webview.run().unwrap();
    });
    tokio::join!(ui_handle, message_handle);
}

fn invoke_handler(
    view: &mut WebView<()>,
    arg: &str,
    sender: UnboundedSender<(RuntimeMessage, Handle<()>)>,
) -> WVResult<()> {
    let handler = view.handle();
    debug!("Argument from runtime: {}", arg);
    let message: RuntimeMessage = serde_json::from_str(arg).unwrap();
    sender.send((message, handler));
    Ok(())
}

async fn message_loop(mut recv: UnboundedReceiver<(RuntimeMessage, Handle<()>)>) {
    loop {
        match recv.recv().await {
            None => {}
            Some(message) => {
                let handler = message.1;
                let message = message.0;
                match message {
                    RuntimeMessage::Login { login, password } => {
                        let client = Arc::clone(CLIENT.get().expect("Client not found"));
                        messages::login(login, password, client, handler).await;
                    }
                    RuntimeMessage::Play { profile } => {
                        let client = Arc::clone(CLIENT.get().expect("Client not found"));
                        messages::play(profile, client, handler).await;
                    }
                    RuntimeMessage::Ready => {
                        messages::ready(handler).await;
                    }
                };
            }
        };
    }
}
