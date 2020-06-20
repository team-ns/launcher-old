use std::thread;

use launcher_api::message::ServerMessage::{Auth, Error as OtherError, ProfileResources};
use launcher_api::message::{AuthMessage, AuthResponse, ClientMessage, ServerMessage};
use launcher_api::message::{Error, ProfileResourcesMessage, ProfileResourcesResponse};
use tokio::sync::mpsc::{Receiver, Sender};
use url::Url;

use crate::config::Config;
use crate::security;
use crate::security::SecurityManager;
use launcher_api::config::Configurable;

pub mod downloader;

pub struct Client {
    out: Sender<String>,
    recv: Receiver<String>,
    security: SecurityManager,
    pub auth_info: Option<AuthInfo>,
    pub config: Config,
}

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

    pub async fn auth(&mut self, login: &str, password: &str) -> Result<AuthResponse, Error> {
        let message = ClientMessage::Auth(AuthMessage {
            login: String::from(login),
            password: self.security.encrypt(password),
        });
        match self.send_sync(message).await {
            Auth(auth) => Ok(auth),
            OtherError(e) => Err(e),
            _ => Err(Error {
                msg: "Auth not found".to_string(),
            }),
        }
    }

    pub async fn get_profile(&mut self, profile: &str) -> Result<ProfileResourcesResponse, Error> {
        let message = ClientMessage::ProfileResources(ProfileResourcesMessage {
            profile: String::from(profile),
        });
        match self.send_sync(message).await {
            ProfileResources(profile) => Ok(profile),
            OtherError(e) => Err(e),
            _ => Err(Error {
                msg: "Profile sync error".to_string(),
            }),
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
