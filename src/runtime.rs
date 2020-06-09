use std::cell::RefCell;
use std::sync::Arc;

use rust_embed::RustEmbed;
use tokio::runtime::{Handle, Runtime};
use tokio::sync::Mutex;
use web_view::{Content, WVResult};

use messages::RuntimeMessage;

use crate::client::WebSocketClient;

mod messages;

#[derive(RustEmbed)]
#[folder = "runtime/"]
struct Asset;

struct Handler {
    ws: WebSocketClient,
}

impl Handler {
    async fn auth(&mut self, login: &str, password: &str) -> bool {
        self.ws.auth(login, password).await.is_ok()
    }
}

pub async fn start() {
    let mut socket: Arc<Mutex<WebSocketClient>> = Arc::new(Mutex::new(
        WebSocketClient::new("ws://127.0.0.1:8080/api/").await,
    ));
    let resources = std::str::from_utf8(&Asset::get("index.html").unwrap().to_mut())
        .unwrap()
        .to_string();
    let mut webview = web_view::builder()
        .title("NSLauncher")
        .content(Content::Html(&resources))
        .size(1000, 600)
        .resizable(false)
        .debug(true)
        .user_data(())
        .invoke_handler(move |view, arg| {
            let handler = view.handle();
            let mut socket = Arc::clone(&socket);
            println!("{}", arg);
            let message: RuntimeMessage = serde_json::from_str(arg).unwrap();
            match message {
                RuntimeMessage::Login { login, password } => {
                    println!("who");
                    tokio::spawn(async move {
                        let mut value = socket.lock().await;
                        let result = value.auth(&login, &password).await;
                        if result.is_ok() {
                            handler.dispatch(|w| {
                                w.eval("app.backend.logined()");
                                let result = result.ok().unwrap();
                                println!("{}, {}", &result.access_token, &result.uuid);
                                Ok(())
                            });
                        }
                    });
                }
                RuntimeMessage::Play { profile } => {}
            }

            Ok(())
        })
        .build()
        .unwrap();

    let value = webview.run().unwrap();
}
