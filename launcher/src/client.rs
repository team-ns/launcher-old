use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde_json::Value;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

use launcher_api::message::{
    AuthMessage, AuthResponse, ClientMessage, ClientRequest, ConnectedMessage, JoinServerMessage,
    ProfileMessage, ProfileResponse, ProfilesInfoMessage, ProfilesInfoResponse, ServerMessage,
    ServerResponse,
};
use launcher_api::message::{Error, ProfileResourcesMessage, ProfileResourcesResponse};
use launcher_api::validation::ClientInfo;

use crate::config::BUNDLE;
use crate::runtime::webview::{EventProxy, WebviewEvent};
use crate::security;
use crate::security::validation::get_os_type;
use crate::security::SecurityManager;

pub mod downloader;

pub struct Client {
    sender: Sender<Vec<u8>>,
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
    pub async fn new(runtime_sender: EventProxy) -> Result<Self> {
        let address: &str = &BUNDLE.websocket;
        let (sender, mut receiver) = Client::connect(address).await?;
        let requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<ServerMessage>>>> =
            Default::default();
        let response_requests = requests.clone();
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    None => {
                        let mut requests = response_requests.lock().await;
                        requests.clear();
                        log::debug!("Websocket channel closed");
                        break;
                    }
                    Some(message) => {
                        let response = bincode::deserialize::<ServerResponse>(&message)
                            .expect("Can't parse response");
                        log::debug!("Server message: {:?}", response);
                        if let Some(request_id) = response.request_id {
                            let mut requests = response_requests.lock().await;
                            if let Some(sender) = requests.remove(&request_id) {
                                sender.send(response.message).expect("Can't send message");
                            }
                        } else if let ServerMessage::Runtime(message) = response.message {
                            match serde_json::from_str::<Value>(&message) {
                                Ok(payload) => {
                                    runtime_sender
                                        .send_event(WebviewEvent::Emit(
                                            "customMessage".to_string(),
                                            payload,
                                        ))
                                        .expect("Can't send message to runtime");
                                }
                                Err(error) => {
                                    log::error!("Can't parse custom message to json: {}", error);
                                }
                            }
                        }
                    }
                }
            }
        });
        Ok(Client {
            security: security::get_manager(),
            sender,
            auth_info: None,
            requests,
        })
    }

    async fn connect(address: &str) -> Result<(Sender<Vec<u8>>, Receiver<Vec<u8>>)> {
        let ws = yarws::Client::new(address)
            .connect()
            .await
            .map_err(|_e| anyhow!("Connection error"))?
            .into_binary();
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

    pub async fn connected(&mut self, client_info: ClientInfo) -> Result<()> {
        let message = ClientMessage::Connected(ConnectedMessage { client_info });
        match self.send_sync(message).await {
            ServerMessage::Empty => Ok(()),
            ServerMessage::Error(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Message not found")),
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

    pub async fn fetch_resources(
        &mut self,
        profile: &str,
        optionals: Vec<String>,
    ) -> Result<ProfileResourcesResponse> {
        let message = ClientMessage::ProfileResources(ProfileResourcesMessage {
            profile: String::from(profile),
            os_type: get_os_type(),
            optionals,
        });
        match self.send_sync(message).await {
            ServerMessage::ProfileResources(profile) => Ok(profile),
            ServerMessage::Error(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Profile resources sync error")),
        }
    }

    pub async fn fetch_profiles(&mut self) -> Result<ProfilesInfoResponse> {
        let message = ClientMessage::ProfilesInfo(ProfilesInfoMessage);
        match self.send_sync(message).await {
            ServerMessage::ProfilesInfo(info) => Ok(info),
            ServerMessage::Error(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Profiles info sync error")),
        }
    }

    pub async fn fetch_profile(
        &mut self,
        profile: &str,
        optionals: Vec<String>,
    ) -> Result<ProfileResponse> {
        let message = ClientMessage::Profile(ProfileMessage {
            profile: String::from(profile),
            optionals,
        });
        match self.send_sync(message).await {
            ServerMessage::Profile(profile) => Ok(profile),
            ServerMessage::Error(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Profile sync error!")),
        }
    }

    pub async fn custom_message(&mut self, message: &str) -> Result<String> {
        let message = ClientMessage::Custom(message.to_string());
        match self.send_sync(message).await {
            ServerMessage::Runtime(result) => Ok(result),
            ServerMessage::Error(error) => Err(anyhow::anyhow!("{}", error.msg)),
            _ => Err(anyhow::anyhow!("Profile sync error!")),
        }
    }

    async fn send_sync(&mut self, msg: ClientMessage) -> ServerMessage {
        let request_uuid = Uuid::new_v4();
        let request = ClientRequest {
            request_id: request_uuid,
            message: msg,
        };
        let (sender, receiver) = tokio::sync::oneshot::channel();
        {
            let mut requests = self.requests.lock().await;
            requests.insert(request_uuid, sender);
        }
        self.sender
            .send(bincode::serialize(&request).expect("Can't serialize client message"))
            .await
            .unwrap_or_else(|_| panic!("Can't send message to closed connection"));
        match receiver.await {
            Ok(message) => message,
            Err(e) => ServerMessage::Error(Error {
                msg: format!("Server Disconnected: {}", e),
            }),
        }
    }
}
