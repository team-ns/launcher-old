use std::sync::Arc;

use anyhow::Result;
use futures::{FutureExt, StreamExt};
use launcher_api::message::{
    AuthMessage, AuthResponse, ClientMessage, Error, ProfileMessage, ProfileResourcesMessage,
    ProfileResourcesResponse, ProfileResponse, ProfilesInfoMessage, ProfilesInfoResponse,
    ServerMessage,
};
use launcher_api::validation::HashedDirectory;
use log::error;
use rand::Rng;
use std::collections::HashMap;
use std::hash::Hash;
use tokio::macros::support::Future;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{mpsc, RwLock};
use warp::filters::ws::{Message, WebSocket};

use crate::security::NativeVersion;
use crate::LaunchServer;

pub struct Client {}

impl Client {
    fn new() -> Self {
        Client {}
    }
}

pub async fn ws_api(ws: WebSocket, server: Arc<RwLock<LaunchServer>>) {
    let (ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::task::spawn(rx.forward(ws_tx).map(|result| {
        if let Err(e) = result {
            error!("Websocket send error: {}", e);
        }
    }));
    let mut client = Client::new();
    while let Some(result) = ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("Websocket error");
                break;
            }
        };
        if let Ok(json) = msg.to_str() {
            println!("{:?}", json.to_string());
            if let Ok(message) = serde_json::from_str::<ClientMessage>(json) {
                match message {
                    ClientMessage::Auth(auth) => {
                        auth.handle(tx.clone(), server.clone(), &mut client).await;
                    }
                    ClientMessage::Profile(profile) => {
                        profile
                            .handle(tx.clone(), server.clone(), &mut client)
                            .await;
                    }
                    ClientMessage::ProfileResources(resources) => {
                        resources
                            .handle(tx.clone(), server.clone(), &mut client)
                            .await;
                    }
                    ClientMessage::ProfilesInfo(profiles_info) => {
                        profiles_info
                            .handle(tx.clone(), server.clone(), &mut client)
                            .await;
                    }
                }
            }
        }
    }
}

async fn send(tx: UnboundedSender<Result<Message, warp::Error>>, f: impl Future<Output = Result<ServerMessage, String>>) {
    let message = match f.await {
        Ok(message) => message,
        Err(e) => ServerMessage::Error(Error {msg : e})
    };
    tx.send(Ok(Message::text(serde_json::to_string(&message).unwrap())));
}

#[async_trait::async_trait]
pub trait Handle {
    async fn handle(
        &self,
        tx: UnboundedSender<Result<Message, warp::Error>>,
        server: Arc<RwLock<LaunchServer>>,
        client: &mut Client,
    );
}

fn get_resource<T>(
    resource: &Option<HashMap<T, HashedDirectory>>,
    key: &T,
) -> Result<HashedDirectory, String>
where
    T: Eq + Hash,
{
    match resource.as_ref().map(|resource| resource.get(&key)).flatten() {
        Some(resource) => Ok(resource.to_owned()),
        None => Err(
            "This profile resource doesn't exist or not synchronized!".to_string()
        ),
    }
}

#[async_trait::async_trait]
impl Handle for ProfileResourcesMessage {
    async fn handle(
        &self,
        tx: UnboundedSender<Result<Message, warp::Error>>,
        server: Arc<RwLock<LaunchServer>>,
        client: &mut Client,
    ) {
        let server = &*server.read().await;
        send(tx, async {
            match server.profiles.get(&self.profile) {
                Some(profile) => {
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
                        assets,
                        natives,
                        jre,
                    }))
                }
                None => Err("This profile doesn't exist!".to_string()),
            }
        }).await;
    }
}

#[async_trait::async_trait]
impl Handle for ProfileMessage {
    async fn handle(
        &self,
        tx: UnboundedSender<Result<Message, warp::Error>>,
        server: Arc<RwLock<LaunchServer>>,
        client: &mut Client,
    ) {
        let server = server.read().await;
        send(tx, async {
            match server.profiles.get(&self.profile) {
                Some(profile) => Ok(ServerMessage::Profile(ProfileResponse {
                    profile: profile.to_owned(),
                })),
                None => Err("This profile doesn't exist!".to_string()),
            }
        }).await;
    }
}

#[async_trait::async_trait]
impl Handle for ProfilesInfoMessage {
    async fn handle(
        &self,
        tx: UnboundedSender<Result<Message, warp::Error>>,
        server: Arc<RwLock<LaunchServer>>,
        client: &mut Client,
    ) {
        let server = server.read().await;
        send(tx, async {
            Ok(ServerMessage::ProfilesInfo(ProfilesInfoResponse {
                profiles_info: server.profiles_info.clone(),
            }))
        }).await;
    }
}

#[async_trait::async_trait]
impl Handle for AuthMessage {
    async fn handle(
        &self,
        tx: UnboundedSender<Result<Message, warp::Error>>,
        server: Arc<RwLock<LaunchServer>>,
        client: &mut Client,
    ) {
        let server = server.read().await;
        //TODO ADD IP FOR LIMITERS
        let ip = "".to_string();
        send(tx, async {
            let password = server.security.decrypt(&self.password)?;
            let result = server.config.auth.auth(&self.login, &password, &ip).await?;
            if result.message.is_none() {
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
                let uuid = result.uuid.unwrap();
                server.config.auth.update_access_token(&uuid, &access_token).await?;
                Ok(ServerMessage::Auth(AuthResponse {
                    uuid: uuid.to_string(),
                    access_token: access_token.to_string(),
                }))
            } else {
                Err(result.message.unwrap())
            }
        }).await;
    }
}