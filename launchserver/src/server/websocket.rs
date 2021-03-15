use std::sync::Arc;

use anyhow::Result;
use futures::{FutureExt, StreamExt};
use launcher_api::message::{
    AuthMessage, AuthResponse, ClientMessage, ClientRequest, Error, JoinServerMessage,
    ProfileMessage, ProfileResourcesMessage, ProfileResourcesResponse, ProfileResponse,
    ProfilesInfoMessage, ProfilesInfoResponse, ServerMessage, ServerResponse,
};
use launcher_api::validation::RemoteDirectory;
use log::debug;
use log::error;
use rand::Rng;
use std::collections::HashMap;
use std::hash::Hash;
use tokio::macros::support::Future;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::filters::ws::{Message, WebSocket};

use crate::security::NativeVersion;
use crate::LaunchServer;
use uuid::Uuid;

pub struct Client {
    _client_id: Uuid,
    #[allow(unused)] // remove when ready ip limiter
    ip: String,
    access_token: Option<String>,
    username: Option<String>,
}

impl Client {
    fn new(ip: &str) -> Self {
        Client {
            _client_id: Uuid::new_v4(),
            ip: ip.to_string(),
            access_token: None,
            username: None,
        }
    }
}

pub async fn ws_api(ws: WebSocket, server: Arc<RwLock<LaunchServer>>, ip: String) {
    let (ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);
    tokio::task::spawn(
        rx.map(|server_message| Ok(map_message(server_message)))
            .forward(ws_tx)
            .map(|result| {
                if let Err(e) = result {
                    error!("Websocket send error: {}", e);
                }
            }),
    );
    let mut client = Client::new(&ip);
    while let Some(result) = ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("Websocket error: {:?}", e);
                break;
            }
        };
        if let Ok(json) = msg.to_str() {
            debug!("Client message: {:?}", json.to_string());
            if let Ok(request) = serde_json::from_str::<ClientRequest>(json) {
                let request_id = request.request_id;
                match request.message {
                    ClientMessage::Auth(auth) => {
                        auth.handle(request_id, tx.clone(), server.clone(), &mut client)
                            .await;
                    }
                    ClientMessage::JoinServer(join) => {
                        join.handle(request_id, tx.clone(), server.clone(), &mut client)
                            .await;
                    }
                    ClientMessage::Profile(profile) => {
                        profile
                            .handle(request_id, tx.clone(), server.clone(), &mut client)
                            .await;
                    }
                    ClientMessage::ProfileResources(resources) => {
                        resources
                            .handle(request_id, tx.clone(), server.clone(), &mut client)
                            .await;
                    }
                    ClientMessage::ProfilesInfo(profiles_info) => {
                        profiles_info
                            .handle(request_id, tx.clone(), server.clone(), &mut client)
                            .await;
                    }
                }
            }
        }
    }
}

fn map_message(message: ServerResponse) -> Message {
    Message::text(serde_json::to_string(&message).unwrap())
}

async fn send(
    tx: UnboundedSender<ServerResponse>,
    request_id: Uuid,
    f: impl Future<Output = Result<ServerMessage>>,
) {
    let message = match f.await {
        Ok(message) => message,
        Err(e) => ServerMessage::Error(Error {
            msg: format!("{}", e),
        }),
    };
    let response = ServerResponse {
        request_id: Some(request_id),
        message,
    };
    tx.send(response).expect("Can't send message to client");
}

#[async_trait::async_trait]
pub trait Handle {
    async fn handle(
        &self,
        request_id: Uuid,
        tx: UnboundedSender<ServerResponse>,
        server: Arc<RwLock<LaunchServer>>,
        client: &mut Client,
    );
}

fn get_resource<T>(
    resource: &Option<HashMap<T, RemoteDirectory>>,
    key: &T,
) -> Result<RemoteDirectory>
where
    T: Eq + Hash,
{
    match resource
        .as_ref()
        .map(|resource| resource.get(&key))
        .flatten()
    {
        Some(resource) => Ok(resource.to_owned()),
        None => Err(anyhow::anyhow!(
            "This profile resource doesn't exist or not synchronized!"
        )),
    }
}

#[async_trait::async_trait]
impl Handle for ProfileResourcesMessage {
    async fn handle(
        &self,
        request_id: Uuid,
        tx: UnboundedSender<ServerResponse>,
        server: Arc<RwLock<LaunchServer>>,
        _client: &mut Client,
    ) {
        let server = &*server.read().await;
        send(tx, request_id, async {
            match server.profiles.get(&self.profile) {
                Some(profile) => {
                    let libraries = get_resource(&server.security.libraries, &self.profile)?;
                    let assets = get_resource(&server.security.assets, &profile.assets)?;
                    let natives = get_resource(
                        &server.security.natives,
                        &NativeVersion {
                            version: profile.version.clone(),
                            os_type: self.os_type.clone(),
                        },
                    )?;
                    let jre = get_resource(&server.security.jres, &self.os_type)?;
                    let profile = get_resource(&server.security.profiles, &self.profile)?;

                    Ok(ServerMessage::ProfileResources(ProfileResourcesResponse {
                        profile,
                        libraries,
                        assets,
                        natives,
                        jre,
                    }))
                }
                None => Err(anyhow::anyhow!("This profile doesn't exist!")),
            }
        })
        .await;
    }
}

#[async_trait::async_trait]
impl Handle for ProfileMessage {
    async fn handle(
        &self,
        request_id: Uuid,
        tx: UnboundedSender<ServerResponse>,
        server: Arc<RwLock<LaunchServer>>,
        _client: &mut Client,
    ) {
        let server = server.read().await;
        send(tx, request_id, async {
            match server.profiles.get(&self.profile) {
                Some(profile) => Ok(ServerMessage::Profile(ProfileResponse {
                    profile: profile.to_owned(),
                })),
                None => Err(anyhow::anyhow!("This profile doesn't exist!")),
            }
        })
        .await;
    }
}

#[async_trait::async_trait]
impl Handle for ProfilesInfoMessage {
    async fn handle(
        &self,
        request_id: Uuid,
        tx: UnboundedSender<ServerResponse>,
        server: Arc<RwLock<LaunchServer>>,
        _client: &mut Client,
    ) {
        let server = server.read().await;
        send(tx, request_id, async {
            Ok(ServerMessage::ProfilesInfo(ProfilesInfoResponse {
                profiles_info: server.profiles_info.clone(),
            }))
        })
        .await;
    }
}

#[async_trait::async_trait]
impl Handle for AuthMessage {
    async fn handle(
        &self,
        request_id: Uuid,
        tx: UnboundedSender<ServerResponse>,
        server: Arc<RwLock<LaunchServer>>,
        client: &mut Client,
    ) {
        let server = server.read().await;
        //TODO ADD IP FOR LIMITERS
        let ip = client.ip.clone();
        send(tx, request_id, async {
            let password = server.security.decrypt(&self.password)?;
            let result = server
                .auth_provider
                .auth(&self.login, &password, &ip)
                .await?;
            let digest = {
                let mut rng = rand::thread_rng();
                md5::compute(format!(
                    "{}{}{}",
                    rng.gen_range(1000000000, 2147483647),
                    rng.gen_range(1000000000, 2147483647),
                    rng.gen_range(0, 9)
                ))
            };
            let access_token = format!("{:x}", digest);
            server
                .auth_provider
                .update_access_token(&result, &access_token)
                .await?;
            client.username = Some(self.login.clone());
            client.access_token = Some(access_token.clone());
            Ok(ServerMessage::Auth(AuthResponse {
                uuid: result.to_string(),
                access_token: access_token.to_string(),
            }))
        })
        .await;
    }
}

#[async_trait::async_trait]
impl Handle for JoinServerMessage {
    async fn handle(
        &self,
        request_id: Uuid,
        tx: UnboundedSender<ServerResponse>,
        server: Arc<RwLock<LaunchServer>>,
        _client: &mut Client,
    ) {
        let server = server.read().await;
        send(tx, request_id, async {
            let provide = &server.auth_provider;
            let entry = provide.get_entry(&self.selected_profile).await;
            match entry {
                Ok(e) => {
                    if e.access_token.is_some() && e.access_token.unwrap().eq(&self.access_token) {
                        provide
                            .update_server_id(&self.selected_profile, &self.server_id)
                            .await?;
                        Ok(ServerMessage::Empty)
                    } else {
                        Ok(ServerMessage::Error(Error {
                            msg: String::from("Access token error"),
                        }))
                    }
                }
                Err(error) => Ok(ServerMessage::Error(Error {
                    msg: format!("{}", error),
                })),
            }
        })
        .await;
    }
}
