use launcher_api::config::Configurable;
use messages::RuntimeMessage;
use rust_embed::RustEmbed;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use web_view::Content;

use crate::client::{AuthInfo, Client};
use crate::game;
use crate::security::validation;

mod messages;

#[derive(RustEmbed)]
#[folder = "runtime/"]
struct Asset;

struct Handler {
    ws: Client,
}

impl Handler {
    async fn auth(&mut self, login: &str, password: &str) -> bool {
        self.ws.auth(login, password).await.is_ok()
    }
}

pub async fn start() {
    let mut socket: Arc<Mutex<Client>> =
        Arc::new(Mutex::new(Client::new("ws://127.0.0.1:8080/api/").await));
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
                    tokio::spawn(async move {
                        let mut value = socket.lock().await;
                        let result = value.auth(&login, &password).await;
                        if result.is_ok() {
                            let result = result.ok().unwrap();
                            value.auth_info = Some(AuthInfo {
                                access_token: result.access_token,
                                uuid: result.uuid,
                                username: login,
                            });
                            handler.dispatch(|w| {
                                w.eval("app.backend.logined()");
                                Ok(())
                            });
                        }
                    });
                }
                RuntimeMessage::Play { profile } => {
                    tokio::spawn(async move {
                        let mut value = socket.lock().await;
                        let resources = value.get_profile(&profile).await;
                        if resources.is_ok() {
                            let resources = resources.ok().unwrap();
                            validation::validate_profile(
                                value.config.game_dir.clone(),
                                profile.clone(),
                                resources.profile,
                                value.config.file_server.clone(),
                            )
                            .await
                            .unwrap();

                            handler.dispatch(|w| {
                                w.exit();
                                Ok(())
                            });

                            let client = game::Client {
                                name: profile.clone(),
                            };
                            game::Client::start(
                                &client,
                                &value.config.game_dir,
                                &value.auth_info.as_ref().unwrap().uuid,
                                &value.auth_info.as_ref().unwrap().access_token,
                                &value.auth_info.as_ref().unwrap().username,
                            );
                        }
                    });
                }
            }

            Ok(())
        })
        .build()
        .unwrap();

    let value = webview.run().unwrap();
}
