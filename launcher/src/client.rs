use anyhow::{anyhow, Result};

use launcher_api::message::{
    AuthMessage, AuthResponse, ClientMessage, JoinServerMessage, ProfileMessage, ProfileResponse,
    ProfilesInfoMessage, ProfilesInfoResponse, ServerMessage,
};
use launcher_api::message::{Error, ProfileResourcesMessage, ProfileResourcesResponse};

use tokio::sync::mpsc::{Receiver, Sender};

use crate::config::CONFIG;

use crate::security;
use crate::security::validation::get_os_type;
use crate::security::SecurityManager;
use uuid::Uuid;

pub mod downloader;

pub struct Client {
    out: Sender<String>,
    recv: Receiver<String>,
    security: SecurityManager,
    pub auth_info: Option<AuthInfo>,
}

#[derive(Clone)]
pub struct AuthInfo {
    pub uuid: String,
    pub access_token: String,
    pub username: String,
}

impl Client {
    pub async fn new() -> Result<Self> {
        let address: &str = &CONFIG.websocket;
        let (s, r) = Client::connect(&address).await?;
        Ok(Client {
            security: security::get_manager(),
            recv: r,
            out: s,
            auth_info: None,
        })
    }

    async fn connect(address: &str) -> Result<(Sender<String>, Receiver<String>)> {
        let ws = yarws::Client::new(address)
            .connect()
            .await
            .map_err(|_e| anyhow!("Connection error"))?
            .into_text();
        Ok(ws.into_channel().await)
    }

    pub async fn get_encrypted_password(&self, password: &str) -> String {
        self.security.encrypt(password)
    }

    pub async fn auth(&mut self, login: &str, password: &str) -> Result<AuthResponse> {
        let message = ClientMessage::Auth(AuthMessage {
            login: String::from(login),
            password: password.to_string(),
        });
        match self.send_sync(message).await {
            ServerMessage::Auth(auth) => Ok(auth),
            ServerMessage::Error(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Auth not found")),
        }
    }

    pub async fn join(&mut self, token: &str, profile: &Uuid, server: &str) -> Result<()> {
        let message = ClientMessage::JoinServer(JoinServerMessage {
            access_token: String::from(token),
            selected_profile: *profile,
            server_id: String::from(server),
        });
        match self.send_sync(message).await {
            ServerMessage::Empty => Ok(()),
            ServerMessage::Error(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Auth not found")),
        }
    }

    pub async fn get_resources(&mut self, profile: &str) -> Result<ProfileResourcesResponse> {
        let message = ClientMessage::ProfileResources(ProfileResourcesMessage {
            profile: String::from(profile),
            os_type: get_os_type(),
        });
        match self.send_sync(message).await {
            ServerMessage::ProfileResources(profile) => Ok(profile),
            ServerMessage::Error(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Profile resources sync error")),
        }
    }

    pub async fn get_profiles(&mut self) -> Result<ProfilesInfoResponse> {
        let message = ClientMessage::ProfilesInfo(ProfilesInfoMessage);
        match self.send_sync(message).await {
            ServerMessage::ProfilesInfo(info) => Ok(info),
            ServerMessage::Error(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Profiles info sync error")),
        }
    }

    pub async fn get_profile(&mut self, profile: &str) -> Result<ProfileResponse> {
        let message = ClientMessage::Profile(ProfileMessage {
            profile: String::from(profile),
        });
        match self.send_sync(message).await {
            ServerMessage::Profile(profile) => Ok(profile),
            ServerMessage::Error(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Profile sync error!")),
        }
    }

    async fn send_sync(&mut self, msg: ClientMessage) -> ServerMessage {
        self.out
            .send(serde_json::to_string(&msg).unwrap())
            .await
            .expect("Can't send message to server");
        match self.recv.recv().await {
            Some(message) => serde_json::from_str(&message).unwrap(),
            None => ServerMessage::Error(Error {
                msg: "Server Disconnected".to_string(),
            }),
        }
    }
}
