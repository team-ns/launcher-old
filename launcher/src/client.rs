use anyhow::{anyhow, Result};

use launcher_api::message::{
    AuthMessage, AuthResponse, ClientMessage, ClientRequest, JoinServerMessage, ProfileMessage,
    ProfileResponse, ProfilesInfoMessage, ProfilesInfoResponse, ServerMessage, ServerResponse,
};
use launcher_api::message::{Error, ProfileResourcesMessage, ProfileResourcesResponse};

use tokio::sync::mpsc::UnboundedSender;

use crate::config::CONFIG;

use crate::security;
use crate::security::validation::get_os_type;
use crate::security::SecurityManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

pub mod downloader;

pub struct Client {
    out: mpsc::Sender<String>,
    security: SecurityManager,
    pub auth_info: Option<AuthInfo>,
    requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<ServerMessage>>>>,
}

#[derive(Clone)]
pub struct AuthInfo {
    pub uuid: String,
    pub access_token: String,
    pub username: String,
}

impl Client {
    pub async fn new(runtime_sender: UnboundedSender<String>) -> Result<Self> {
        let address: &str = &CONFIG.websocket;
        let (out, mut r) = Client::connect(&address).await?;
        let requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<ServerMessage>>>> =
            Arc::new(Mutex::new(HashMap::default()));
        let response_requests = requests.clone();
        tokio::spawn(async move {
            loop {
                match r.recv().await {
                    None => {
                        break;
                    }
                    Some(m) => {
                        let response = serde_json::from_str::<ServerResponse>(&m)
                            .expect("Can't parse response");

                        if let Some(request_id) = response.request_id {
                            let mut requests = response_requests.lock().await;
                            if let Some(sender) = requests.remove(&request_id) {
                                sender.send(response.message).expect("Can't send message");
                            }
                        } else if let ServerMessage::Runtime(message) = response.message {
                            runtime_sender.send(message).expect("Can't send message");
                        }
                    }
                }
            }
        });
        Ok(Client {
            security: security::get_manager(),
            out,
            auth_info: None,
            requests,
        })
    }

    async fn connect(address: &str) -> Result<(mpsc::Sender<String>, mpsc::Receiver<String>)> {
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
        let request_uuid = uuid::Uuid::new_v4();
        let request = ClientRequest {
            request_id: request_uuid,
            message: msg,
        };
        let (sender, receiver) = tokio::sync::oneshot::channel();
        {
            let mut requests = self.requests.lock().await;
            requests.insert(request_uuid, sender);
        }
        self.out
            .send(serde_json::to_string(&request).unwrap())
            .await
            .expect("Can't send message to server");
        match receiver.await {
            Ok(message) => message,
            Err(e) => ServerMessage::Error(Error {
                msg: format!("Server Disconnected: {}", e),
            }),
        }
    }
}
