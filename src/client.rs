use anyhow::Result;
use launcher_api::config::Configurable;
use launcher_api::message::ServerMessage::{Auth, Error as OtherError, Profile, ProfileResources};
use launcher_api::message::{
    AuthMessage, AuthResponse, ClientMessage, ProfileMessage, ProfileResponse, ServerMessage,
};
use launcher_api::message::{Error, ProfileResourcesMessage, ProfileResourcesResponse};
use launcher_api::validation::OsType;
use std::thread;
use tokio::sync::mpsc::{Receiver, Sender};
use url::Url;

use crate::config::Config;
use crate::security;
use crate::security::SecurityManager;

pub mod downloader;

pub struct Client {
    out: Sender<String>,
    recv: Receiver<String>,
    security: SecurityManager,
    pub auth_info: Option<AuthInfo>,
    pub config: Config,
}

#[derive(Clone)]
pub struct AuthInfo {
    pub uuid: String,
    pub access_token: String,
    pub username: String,
}

impl Client {
    pub async fn new(address: &str) -> Self {
        let ws = yarws::connect(address, yarws::log::config())
            .await
            .unwrap()
            .into_text();
        let (s, r) = ws.into_channel().await;
        Client {
            security: security::get_manager(),
            recv: r,
            out: s,
            auth_info: None,
            config: Config::get_config(
                dirs::config_dir()
                    .unwrap()
                    .join("nsl")
                    .join("config.json")
                    .as_path(),
            )
            .unwrap(),
        }
    }

    pub async fn auth(&mut self, login: &str, password: &str) -> Result<AuthResponse> {
        let message = ClientMessage::Auth(AuthMessage {
            login: String::from(login),
            password: self.security.encrypt(password),
        });
        match self.send_sync(message).await {
            Auth(auth) => Ok(auth),
            OtherError(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Auth not found")),
        }
    }

    pub async fn get_resources(&mut self, profile: &str) -> Result<ProfileResourcesResponse> {
        #[cfg(all(target_os = "linux", target_arch = "x86"))]
        let os_type = OsType::LinuxX32;
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let os_type = OsType::LinuxX64;
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let os_type = OsType::MacOSX64;
        #[cfg(all(target_os = "windows", target_arch = "x86"))]
        let os_type = OsType::WindowsX32;
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        let os_type = OsType::WindowsX64;

        let message = ClientMessage::ProfileResources(ProfileResourcesMessage {
            profile: String::from(profile),
            os_type,
        });
        match self.send_sync(message).await {
            ProfileResources(profile) => Ok(profile),
            OtherError(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Profile resources sync error")),
        }
    }

    pub async fn get_profile(&mut self, profile: &str) -> Result<ProfileResponse> {
        let message = ClientMessage::Profile(ProfileMessage {
            profile: String::from(profile),
        });
        match self.send_sync(message).await {
            Profile(profile) => Ok(profile),
            OtherError(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Profile sync error!")),
        }
    }

    async fn send_sync(&mut self, msg: ClientMessage) -> ServerMessage {
        self.out
            .send(serde_json::to_string(&msg).unwrap())
            .await
            .unwrap();
        match self.recv.recv().await {
            Some(message) => serde_json::from_str(&message).unwrap(),
            None => ServerMessage::Error(Error {
                msg: "what".to_string(),
            }),
        }
    }
}
