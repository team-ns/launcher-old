use std::thread;

use launcher_api::message::Error;
use launcher_api::message::ServerMessage::{Auth, Error as OtherError};
use launcher_api::message::{AuthMessage, AuthResponse, ClientMessage, ServerMessage};
use tokio::sync::mpsc::{Receiver, Sender};
use url::Url;

use crate::security;
use crate::security::SecurityManager;

pub struct WebSocketClient {
    out: Sender<String>,
    recv: Receiver<String>,
    security: SecurityManager,
}

impl WebSocketClient {
    pub async fn new(address: &str) -> Self {
        let ws = yarws::connect(address, yarws::log::config())
            .await
            .unwrap()
            .into_text();
        let (s, r) = ws.into_channel().await;
        WebSocketClient {
            security: security::get_manager(),
            recv: r,
            out: s,
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
