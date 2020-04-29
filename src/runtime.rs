use rust_embed::RustEmbed;

use crate::client::WebSocketClient;
use std::cell::RefCell;
use std::sync::Arc;
use tokio::runtime::{Handle, Runtime};
use tokio::sync::Mutex;
use web_view::{Content, WVResult};

mod resources;

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
        .size(800, 600)
        .resizable(false)
        .debug(true)
        .user_data(())
        .invoke_handler(move |view, arg| {
            let handler = view.handle();
            println!("хто");
            let mut socket = Arc::clone(&socket);
            tokio::spawn(async move {
                let mut value = socket.lock().await;
                if (*value).auth("Test", "test").await.is_ok() {
                    handler.dispatch(|w| {
                        w.eval("result()");
                        Ok(())
                    });
                }
            });
            Ok(())
        })
        .build()
        .unwrap();

    let value = webview.run().unwrap();
}
