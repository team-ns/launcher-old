use crate::client::{AuthInfo, Client};
use crate::game;
use crate::game::auth::{CHANNEL_GET, CHANNEL_SEND};
use crate::runtime::{CLIENT, PLAYING};
use crate::security::validation;
use anyhow::Result;
use log::error;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::time::Duration;

use crate::config::{Settings, SETTINGS};

use nfd2::Response;
use path_slash::PathBufExt;

use crate::runtime::webview::{EventProxy, WebviewEvent};
use crate::security::validation::get_os_type;
use launcher_api::validation::ClientInfo;
use notify::EventKind;
use std::{env, fs};
use sysinfo::SystemExt;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
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
    handler: EventProxy,
) -> Result<()> {
    let response = client.auth(login, password).await?;
    client.auth_info = Some(AuthInfo {
        access_token: response.access_token,
        uuid: response.uuid,
        username: login.to_string(),
    });
    let profiles = client.get_profiles().await?;
    let json = serde_json::to_string(&profiles.profiles_info)?;
    handler.send_event(WebviewEvent::DispatchScript(format!(
        "app.backend.logined(`{}`)",
        json
    )))?;
    Ok(())
}

pub async fn ready(handler: EventProxy, sender: UnboundedSender<String>) -> Result<()> {
    match Client::new(sender).await {
        Ok(mut c) => {
            let client_info = ClientInfo {
                os_type: get_os_type(),
            };
            c.connected(client_info).await?;
            CLIENT
                .set(Arc::new(Mutex::new(c)))
                .map_err(|_| anyhow::anyhow!("Can't update client"))?;
            let settings = match Settings::load() {
                Ok(s) => s,
                Err(e) => {
                    log::debug!("Settings error: {}", e);
                    let s = Settings::default();
                    s.save()?;
                    s
                }
            };
            SETTINGS
                .set(Arc::new(Mutex::new(settings.clone())))
                .expect("Can't update settings");
            let settings_handler = handler.clone();
            update_settings(&settings, settings_handler).await?;
            if settings.save_data {
                let login = &settings.last_name.expect("Can't get login");
                let password = &settings.saved_password.expect("Can't get saved password");
                let mut client = CLIENT.get().expect("Can't get client").lock().await;
                let login_handler = handler.clone();
                let login_result = login_user(&mut client, login, password, login_handler).await;
                if login_result.is_err() {
                    let mut current_settings =
                        SETTINGS.get().expect("Can't take settings").lock().await;
                    current_settings.last_name = None;
                    current_settings.saved_password = None;
                    current_settings.save_data = false;
                    current_settings.save()?;
                    login_result?;
                }
            }
            let mut system = sysinfo::System::new_all();
            system.refresh_all();
            let max_ram = system.get_total_memory() / 1024;
            handler.send_event(WebviewEvent::DispatchScript(format!(
                "app.backend.ready({})",
                max_ram
            )))?;
        }
        Err(e) => {
            handler.send_event(WebviewEvent::DispatchScript(format!(
                r#"app.backend.error("{}")"#,
                e.to_string().replace(r#"""#, r#"""#)
            )))?;
            tokio::time::sleep(Duration::from_secs(10)).await;
            handler.send_event(WebviewEvent::Exit)?;
        }
    }
    Ok(())
}

pub async fn login(
    login: String,
    password: String,
    remember: bool,
    socket: Arc<Mutex<Client>>,
    handler: EventProxy,
) -> Result<()> {
    let mut client = socket.lock().await;
    let password = client.get_encrypted_password(&password).await;
    let handler = handler.clone();
    login_user(&mut client, &login, &password, handler).await?;
    let mut current_settings = SETTINGS.get().expect("Can't take settings").lock().await;
    if remember {
        current_settings.last_name = Some(login.clone());
        current_settings.saved_password = Some(password.clone());
        current_settings.save_data = true;
        current_settings.save()?;
    } else if current_settings.save_data {
        current_settings.last_name = None;
        current_settings.saved_password = None;
        current_settings.save_data = false;
        current_settings.save()?;
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
    handler: EventProxy,
    socket: Arc<Mutex<Client>>,
    profile: String,
) -> Result<()> {
    let optionals = SETTINGS
        .get()
        .expect("Can't get settings")
        .lock()
        .await
        .get_optionals(&profile);
    let (resources, profile, auth_info) = {
        let mut client = socket.lock().await;
        let resources = client.get_resources(&profile, optionals.clone()).await?;
        let profile = client.get_profile(&profile, optionals).await?.profile;
        let auth_info = client.auth_info.clone();
        (resources, profile, auth_info)
    };
    let (game_dir, ram) = {
        let settings = SETTINGS.get().expect("Can't get settings").lock().await;
        let game_dir = settings.game_dir.clone();
        let ram = settings.ram;
        (game_dir, ram)
    };
    let remote_directory = validation::new_remote_directory(resources);
    let validate_handler = handler.clone();
    let watcher =
        validation::validate_profile(&profile, &remote_directory, validate_handler).await?;
    PLAYING.set(()).expect("Can't set playing status");
    let jvm = game::create_jvm(profile.clone(), &game_dir, ram)?;
    let watcher_handle: tokio::task::JoinHandle<Result<()>> =
        tokio::task::spawn_blocking(move || loop {
            let event = watcher.receiver.recv()??;
            if let EventKind::Modify(_) = event.kind {
                for path in event.paths {
                    if path.is_file() {
                        error!("Directory {:?}", remote_directory);
                        if remote_directory.contains_key(&path) {
                            let remote_file = &remote_directory[&path];
                            let hashed_file = &validation::create_hashed_file(&path)?;
                            if hashed_file != remote_file {
                                return Err(anyhow::anyhow!("Forbidden modification: {:?}", path));
                            }
                        } else {
                            return Err(anyhow::anyhow!("Unknown file: {:?}", path));
                        }
                    }
                }
            }
        });
    let game_handle = tokio::task::spawn_blocking(move || {
        if let Some(info) = auth_info {
            handler.send_event(WebviewEvent::HideWindow)?;
            game::start(jvm, profile, info, &game_dir)?;
        } else {
            return Err(anyhow::anyhow!("Start game before auth!"));
        }
        Ok(())
    });
    let join_handle = tokio::spawn(async {
        loop {
            let (token, profile, server) = CHANNEL_GET.1.lock().unwrap().recv().unwrap();
            let join_result = {
                let mut client = CLIENT.get().unwrap().lock().await;
                client.join(&token, &profile, &server).await
            };
            match join_result {
                Err(e) => CHANNEL_SEND.0.lock().unwrap().send(format!("{}", e)),
                _ => CHANNEL_SEND.0.lock().unwrap().send("".to_string()),
            }
            .expect("Can't send join request");
        }
    });
    let game_handle = futures::future::try_join(game_handle, join_handle);
    tokio::select! {
        watch_result = watcher_handle => {
            if let Err(e) = watch_result? {
                error!("Game stopped! Cause: {}", e);
                std::process::exit(-1);
            }
        }
        game_result = game_handle => {
            game_result?.0?;
        }

    }
    Ok(())
}

pub async fn select_game_dir(handler: EventProxy) -> Result<()> {
    let mut current_settings = SETTINGS
        .get()
        .expect("Can't take settings")
        .try_lock()
        .map_err(|_e| anyhow::anyhow!("Вы уже выбираете папку!"))?;
    let response = nfd2::open_pick_folder(None)?;
    if let Response::Okay(folder) = response {
        current_settings.game_dir = folder.to_slash_lossy();
        current_settings.save()?;
        update_settings(&current_settings, handler).await?;
    }
    Ok(())
}

pub async fn save_settings(settings: Settings, handler: EventProxy) -> Result<()> {
    settings.save()?;
    update_settings(&settings, handler).await?;
    let mut current_settings = SETTINGS.get().expect("Can't take settings").lock().await;
    current_settings.update(&settings)?;
    Ok(())
}

pub async fn update_settings(settings: &Settings, handler: EventProxy) -> Result<()> {
    let settings = settings.clone();
    fs::create_dir_all(&settings.game_dir)?;
    env::set_current_dir(&settings.game_dir)?;
    let json = serde_json::to_string(&settings)?;
    handler.send_event(WebviewEvent::DispatchScript(format!(
        r#"app.backend.settings('{}')"#,
        json.replace(r#"""#, r#"""#)
    )))?;
    Ok(())
}
