use std::sync::Arc;
use std::{env, fs};

use anyhow::Result;
use native_dialog::FileDialog;
use notify::EventKind;
use path_slash::PathBufExt;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use launcher_api::profile::ProfileInfo;
use launcher_api::validation::ClientInfo;

use crate::client::{AuthInfo, Client};
use crate::config::{Settings, SETTINGS};
use crate::game;
use crate::game::auth::{CHANNEL_GET, CHANNEL_SEND};
use crate::runtime::arg::InvokeResolver;
use crate::runtime::webview::{EventProxy, WebviewEvent};
use crate::runtime::{CLIENT, PLAYING};
use crate::security::validation;
use crate::security::validation::get_os_type;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
    Ready,
    #[serde(rename_all = "camelCase")]
    Login {
        username: String,
        password: String,
        remember_me: bool,
    },
    Logout,
    Play {
        profile: String,
    },
    SelectGameDir,
    SaveSettings {
        settings: Settings,
    },
    SendCustomMessage {
        message: String,
    },
}

impl Cmd {
    pub async fn run(self, resolver: InvokeResolver, proxy: EventProxy) {
        match self {
            Cmd::Ready => resolver.resolve_result(ready(proxy).await),
            Cmd::Login {
                username,
                password,
                remember_me,
            } => {
                let client = Arc::clone(CLIENT.get().expect("Client not found"));
                resolver.resolve_result(login(username, password, remember_me, client).await)
            }
            Cmd::Logout => {
                let client = Arc::clone(CLIENT.get().expect("Client not found"));
                resolver.resolve_result(logout(client).await)
            }
            Cmd::Play { profile } => {
                let client = Arc::clone(CLIENT.get().expect("Client not found"));
                resolver.resolve_result(start_client(proxy, client, profile).await)
            }
            Cmd::SelectGameDir => resolver.resolve_result(select_game_dir().await),
            Cmd::SaveSettings { settings } => {
                resolver.resolve_result(save_settings(settings).await)
            }
            Cmd::SendCustomMessage { message } => {
                let client = Arc::clone(CLIENT.get().expect("Client not found"));
                resolver.resolve_result(custom_message(message, client).await)
            }
        };
    }
}

async fn custom_message(message: String, socket: Arc<Mutex<Client>>) -> Result<String> {
    let mut client = socket.lock().await;
    Ok(client.custom_message(&message).await?)
}

async fn login_user(client: &mut Client, login: &str, password: &str) -> Result<Vec<ProfileInfo>> {
    let response = client.auth(login, password).await?;
    client.auth_info = Some(AuthInfo {
        access_token: response.access_token,
        uuid: response.uuid,
        username: login.to_string(),
    });
    let profiles = client.get_profiles().await?;
    Ok(profiles.profiles_info)
}

#[derive(Serialize, Deserialize, Debug)]
struct ReadyResponse {
    profiles: Option<Vec<ProfileInfo>>,
    settings: Settings,
}

async fn ready(handler: EventProxy) -> Result<ReadyResponse> {
    let client_handler = handler.clone();
    match Client::new(client_handler).await {
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
            let settings = update_settings(&settings).await?;
            let ready_settings = settings.clone();
            let profiles = if settings.save_data {
                let login = &settings.last_name.expect("Can't get login");
                let password = &settings.saved_password.expect("Can't get saved password");
                let mut client = CLIENT.get().expect("Can't get client").lock().await;
                let profiles = {
                    let login_result = login_user(&mut client, login, password).await;
                    match login_result {
                        Ok(profiles) => profiles,
                        Err(e) => {
                            let mut current_settings =
                                SETTINGS.get().expect("Can't take settings").lock().await;
                            current_settings.last_name = None;
                            current_settings.saved_password = None;
                            current_settings.save_data = false;
                            current_settings.save()?;
                            return Err(e);
                        }
                    }
                };
                Some(profiles)
            } else {
                None
            };
            let ready_response = ReadyResponse {
                profiles,
                settings: ready_settings,
            };
            Ok(ready_response)
        }
        Err(e) => Err(e),
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct LoginResponse {
    profiles: Vec<ProfileInfo>,
}

async fn login(
    login: String,
    password: String,
    remember: bool,
    socket: Arc<Mutex<Client>>,
) -> Result<LoginResponse> {
    let mut client = socket.lock().await;
    let password = client.get_encrypted_password(&password).await;
    let profiles = login_user(&mut client, &login, &password).await?;
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
    Ok(LoginResponse { profiles })
}

async fn logout(client: Arc<Mutex<Client>>) -> Result<()> {
    let mut client = client.lock().await;
    client.auth_info = None;
    let mut current_settings = SETTINGS.get().expect("Can't take settings").lock().await;
    current_settings.save_data = false;
    current_settings.last_name = None;
    current_settings.saved_password = None;
    current_settings.save()?;
    Ok(())
}

async fn start_client(
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
                        log::error!("Directory {:?}", remote_directory);
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
            let _x = match join_result {
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
                log::error!("Game stopped! Cause: {}", e);
                std::process::exit(-1);
            }
        }
        game_result = game_handle => {
            game_result?.0?;
        }

    }
    Ok(())
}

async fn select_game_dir() -> Result<Settings> {
    let mut current_settings = SETTINGS
        .get()
        .expect("Can't take settings")
        .try_lock()
        .map_err(|_e| anyhow::anyhow!("Вы уже выбираете папку!"))?;
    let path = FileDialog::new()
        .set_location(&current_settings.game_dir)
        .show_open_single_dir();
    if let Ok(Some(folder)) = path {
        current_settings.game_dir = folder.to_slash_lossy();
        current_settings.save()?;
        Ok(update_settings(&current_settings).await?)
    } else {
        Err(anyhow::anyhow!("Can't find folder"))
    }
}

async fn save_settings(settings: Settings) -> Result<Settings> {
    settings.save()?;
    let mut current_settings = SETTINGS.get().expect("Can't take settings").lock().await;
    current_settings.update(&settings)?;
    Ok(update_settings(&settings).await?)
}

async fn update_settings(settings: &Settings) -> Result<Settings> {
    let settings = settings.clone();
    fs::create_dir_all(&settings.game_dir)?;
    env::set_current_dir(&settings.game_dir)?;
    Ok(settings)
}
