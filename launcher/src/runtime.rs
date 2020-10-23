use anyhow::Result;
use messages::RuntimeMessage;
use std::sync::Arc;
use tokio::sync::Mutex;
use web_view::{Content, Handle};

use crate::client::{AuthInfo, Client};
use crate::game;
use crate::security::validation;

mod messages;

#[macro_export]
macro_rules! handle_error {
    ($handler:expr, $result:expr) => {
        if let Err(error) = $result {
            $handler.dispatch(move |w| {
                w.eval(&format!("app.backend.error('{}')", error));
                Ok(())
            });
        }
    };
}

pub async fn start() {
    let socket: Arc<Mutex<Client>> =
        Arc::new(Mutex::new(Client::new("ws://127.0.0.1:9090/api/").await));
    let webview = web_view::builder()
        .title("NSLauncher")
        .content(Content::Html(include_str!("../runtime/index.html")))
        .size(1000, 600)
        .resizable(false)
        .debug(true)
        .user_data(())
        .invoke_handler(move |view, arg| {
            let handler = view.handle();
            let socket = Arc::clone(&socket);
            let message: RuntimeMessage = serde_json::from_str(arg).unwrap();
            match message {
                RuntimeMessage::Login { login, password } => {
                    tokio::spawn(async move {
                        let mut client = socket.lock().await;
                        match client.auth(&login, &password).await {
                            Ok(response) => {
                                client.auth_info = Some(AuthInfo {
                                    access_token: response.access_token,
                                    uuid: response.uuid,
                                    username: login,
                                });
                                handler.dispatch(|w| {
                                    w.eval("app.backend.logined()");
                                    Ok(())
                                });
                            }
                            Err(error) => {
                                handler.dispatch(move |w| {
                                    w.eval(&format!("app.backend.error('{}')", error));
                                    Ok(())
                                });
                            }
                        }
                    });
                }
                RuntimeMessage::Play { profile } => {
                    tokio::spawn(async move {
                        async fn start_client(
                            handler: Handle<()>,
                            socket: Arc<Mutex<Client>>,
                            profile: String,
                        ) -> Result<()> {
                            let mut client = socket.lock().await;
                            let resources = client.get_resources(&profile).await?;
                            validation::validate_profile(
                                client.config.game_dir.clone(),
                                profile.clone(),
                                resources,
                                client.config.file_server.clone(),
                                handler.clone(),
                            )
                            .await?;
                            let profile = client.get_profile(&profile).await?.profile;
                            let jvm = game::create_jvm(profile.clone(), &client.config.game_dir)?;
                            if let Some(info) = client.auth_info.clone() {
                                //jvm watcher start
                                handler.dispatch(|w| {
                                    w.exit();
                                    Ok(())
                                });
                                game::start(jvm, profile, info, &client.config.game_dir)?
                            } else {
                                return Err(anyhow::anyhow!("Start game before auth!"));
                            }
                            Ok(())
                        }
                        handle_error!(
                            handler,
                            start_client(handler.clone(), socket, profile).await
                        );
                    });
                }
            };
            Ok(())
        })
        .build()
        .unwrap();

    let value = webview.run().unwrap();
}
