use crate::client::{AuthInfo, Client};
use crate::game;
use crate::game::auth::{CHANNEL_GET, CHANNEL_SEND};
use crate::runtime::CLIENT;
use crate::security::validation;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::time::Duration;

use web_view::Handle;

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

#[derive(Serialize, Deserialize)]
pub enum RuntimeMessage {
    Ready,
    Login { login: String, password: String },
    Play { profile: String },
}

pub async fn ready(handler: Handle<()>) {
    match Client::new().await {
        Ok(c) => {
            CLIENT.set(Arc::new(Mutex::new(c)));
        }
        Err(e) => {
            handler.dispatch(move |w| {
                w.eval(&format!(
                    r#"app.backend.error("{}")"#,
                    e.to_string().replace(r#"""#, r#"""#)
                ));
                Ok(())
            });
            tokio::time::delay_for(Duration::from_secs(10)).await;
            handler.dispatch(move |w| {
                w.exit();
                Ok(())
            });
        }
    }
}

pub async fn login(
    login: String,
    password: String,
    socket: Arc<Mutex<Client>>,
    handler: Handle<()>,
) {
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
                w.eval(&format!(
                    r#"app.backend.error("{}")"#,
                    error.to_string().replace(r#"""#, r#"""#)
                ));
                Ok(())
            });
        }
    }
}

pub async fn play(profile: String, socket: Arc<Mutex<Client>>, handler: Handle<()>) {
    handle_error!(
        handler,
        start_client(handler.clone(), socket, profile).await
    );
}

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
    let game_dir = client.config.game_dir.clone();
    let auth_info = client.auth_info.clone();
    drop(client);
    let game_handle = tokio::task::spawn_blocking(move || {
        let jvm = game::create_jvm(profile.clone(), &game_dir)?;
        if let Some(info) = auth_info {
            //jvm watcher start
            handler.dispatch(|w| {
                w.exit();
                Ok(())
            });
            game::start(jvm, profile, info, &game_dir)?;
        } else {
            return Err(anyhow::anyhow!("Start game before auth!"));
        }
        Ok(())
    });
    let join_handle = tokio::spawn(async {
        loop {
            let (token, profile, server) = CHANNEL_GET.1.lock().unwrap().recv().unwrap();
            let mut client = CLIENT.get().unwrap().lock().await;
            match client.join(&token, &profile, &server).await {
                Err(e) => CHANNEL_SEND.0.lock().unwrap().send(format!("{}", e)),
                _ => CHANNEL_SEND.0.lock().unwrap().send("".to_string()),
            };
        }
    });
    tokio::join!(join_handle, game_handle);
    Ok(())
}
