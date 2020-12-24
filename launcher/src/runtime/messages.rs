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

use crate::config::{Settings, CONFIG, SETTINGS};

use nfd2::Response;
use path_slash::PathBufExt;

use web_view::Handle;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeMessage {
    Ready,
    #[serde(rename_all = "camelCase")]
    Login {
        login: String,
        password: String,
        remember_me: bool,
    },
    Logout,
    Play {
        profile: String,
    },
    SelectGameDir,
    SaveSettings(Settings),
}

pub async fn login_user(
    client: &mut Client,
    login: &str,
    password: &str,
    handler: Handle<()>,
) -> Result<()> {
    let response = client.auth(login, password).await?;
    client.auth_info = Some(AuthInfo {
        access_token: response.access_token,
        uuid: response.uuid,
        username: login.to_string(),
    });
    let profiles = client.get_profiles().await?;
    let json = serde_json::to_string(&profiles.profiles_info)?;
    handler.dispatch(move |w| {
        w.eval(&format!(
            r#"app.backend.logined('{}')"#,
            json.to_string().replace(r#"""#, r#"""#)
        ));
        Ok(())
    })?;
    Ok(())
}

pub async fn ready(handler: Handle<()>) -> Result<()> {
    match Client::new().await {
        Ok(c) => {
            CLIENT.set(Arc::new(Mutex::new(c)));
            let settings = match Settings::load() {
                Ok(s) => s,
                Err(_e) => {
                    let s = Settings::default();
                    s.save();
                    s
                }
            };
            update_settings(&settings, handler.clone()).await;
            SETTINGS.set(Arc::new(Mutex::new(settings.clone())));
            if settings.save_data {
                let login = &settings.last_name.expect("Can't get login");
                let password = &settings.saved_password.expect("Can't get saved password");
                let mut client = CLIENT.get().expect("Can't get client").lock().await;
                login_user(&mut client, login, password, handler.clone()).await?;
            }
            handler.dispatch(|w| {
                w.eval("app.backend.ready()");
                Ok(())
            });
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
    Ok(())
}

pub async fn login(
    login: String,
    password: String,
    remember: bool,
    socket: Arc<Mutex<Client>>,
    handler: Handle<()>,
) -> Result<()> {
    let mut client = socket.lock().await;
    let password = client.get_encrypted_password(&password).await;
    login_user(&mut client, &login, &password, handler.clone()).await?;
    if remember {
        let mut current_settings = SETTINGS.get().expect("Can't take settings").lock().await;
        current_settings.last_name = Some(login.clone());
        current_settings.saved_password = Some(password.clone());
        current_settings.save_data = true;
        current_settings.save();
    }
    Ok(())
}

pub async fn logout(client: Arc<Mutex<Client>>) -> Result<()> {
    let mut client = client.lock().await;
    client.auth_info = None;
    let mut current_settings = SETTINGS.get().expect("Can't take settings").lock().await;
    current_settings.save_data = false;
    current_settings.last_name = None;
    current_settings.saved_password = None;
    current_settings.save()?;
    Ok(())
}

pub async fn start_client(
    handler: Handle<()>,
    socket: Arc<Mutex<Client>>,
    profile: String,
) -> Result<()> {
    let mut client = socket.lock().await;
    let resources = client.get_resources(&profile).await?;
    let settings = SETTINGS.get().expect("Can't get settings").lock().await;
    let game_dir = settings.game_dir.clone();
    validation::validate_profile(
        game_dir.clone(),
        profile.clone(),
        resources,
        CONFIG.file_server.clone(),
        handler.clone(),
    )
    .await?;
    let profile = client.get_profile(&profile).await?.profile;
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
    tokio::try_join!(game_handle, join_handle);
    Ok(())
}

pub async fn select_game_dir(handler: Handle<()>) -> Result<()> {
    let mut current_settings = SETTINGS
        .get()
        .expect("Can't take settings")
        .try_lock()
        .map_err(|_e| anyhow::anyhow!("Вы уже выбираете папку!"))?;
    let response = nfd2::open_pick_folder(None)?;
    match response {
        Response::Okay(folder) => {
            current_settings.game_dir = folder.to_slash_lossy();
            current_settings.save();
            update_settings(&current_settings, handler).await;
        }
        _ => {}
    }
    Ok(())
}

pub async fn save_settings(settings: Settings, handler: Handle<()>) -> Result<()> {
    settings.save()?;
    update_settings(&settings, handler).await;
    SETTINGS.set(Arc::new(Mutex::new(settings)));
    Ok(())
}

pub async fn update_settings(settings: &Settings, handler: Handle<()>) -> Result<()> {
    let settings = settings.clone();
    let json = serde_json::to_string(&settings)?;
    handler.dispatch(move |w| {
        w.eval(&format!(
            r#"app.backend.settings('{}')"#,
            json.to_string().replace(r#"""#, r#"""#)
        ));
        Ok(())
    });
    Ok(())
}
