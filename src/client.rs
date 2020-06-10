use std::thread;

use launcher_api::message::Error;
use launcher_api::message::ServerMessage::{Auth, Error as OtherError};
use launcher_api::message::{AuthMessage, AuthResponse, ClientMessage, ServerMessage};
use tokio::sync::mpsc::{Receiver, Sender};
use url::Url;

use crate::security;
use crate::security::SecurityManager;
use crate::config::Config;
use launcher_api::config::Configurable;


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
                dirs::config_dir().unwrap()
                    .join("nsl")
                    .join("config.json")
                    .as_path()
            ).unwrap(),
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
