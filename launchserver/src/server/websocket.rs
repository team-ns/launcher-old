use std::sync::Arc;

use anyhow::Result;
use futures::{FutureExt, StreamExt};
use launcher_api::message::{
    AuthMessage, AuthResponse, ClientMessage, Error, JoinServerMessage, ProfileMessage,
    ProfileResourcesMessage, ProfileResourcesResponse, ProfileResponse, ProfilesInfoMessage,
    ProfilesInfoResponse, ServerMessage,
};
use launcher_api::validation::{ClientInfo, RemoteDirectory, RemoteDirectoryExt};
use log::debug;
use log::error;
use std::collections::HashMap;
use std::hash::Hash;
use tokio::macros::support::Future;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::filters::ws::{Message, WebSocket};

use crate::security::{NativeVersion, SecurityManager};
use crate::LaunchServer;

use launcher_api::optional::{Location, Optional};
use launcher_api::profile::ProfileInfo;

pub struct Client {
    #[allow(unused)] // remove when ready ip limiter
    ip: String,
    access_token: Option<String>,
    username: Option<String>,
    client_info: Option<ClientInfo>,
}

impl Client {
    fn new(ip: &str) -> Self {
        Client {
            ip: ip.to_string(),
            access_token: None,
            username: None,
            client_info: None,
        }
    }
}

pub async fn ws_api(ws: WebSocket, server: Arc<RwLock<LaunchServer>>, ip: String) {
    let (ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);
    tokio::task::spawn(rx.forward(ws_tx).map(|result| {
        if let Err(e) = result {
            error!("Websocket send error: {}", e);
        }
    }));
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
            if let Ok(message) = serde_json::from_str::<ClientMessage>(json) {
                match message {
                    ClientMessage::Auth(auth) => {
                        auth.handle(tx.clone(), server.clone(), &mut client).await;
                    }
                    ClientMessage::JoinServer(join) => {
                        join.handle(tx.clone(), server.clone(), &mut client).await;
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

async fn send(
    tx: UnboundedSender<Result<Message, warp::Error>>,
    f: impl Future<Output = Result<ServerMessage>>,
) {
    let message = match f.await {
        Ok(message) => message,
        Err(e) => ServerMessage::Error(Error {
            msg: format!("{}", e),
        }),
    };
    tx.send(Ok(Message::text(serde_json::to_string(&message).unwrap())))
        .expect("Can't send message to client");
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
        tx: UnboundedSender<Result<Message, warp::Error>>,
        server: Arc<RwLock<LaunchServer>>,
        client: &mut Client,
    ) {
        let server = &*server.read().await;
        send(tx, async {
            match server.profiles_data.get(&self.profile) {
                Some(profile_data) => {
                    let info = client.client_info.as_ref().unwrap();
                    let files = profile_data
                        .profile_info
                        .get_irrelevant_optionals(info, &self.optionals)
                        .map(Optional::get_files)
                        .flatten()
                        .collect::<HashMap<_, _>>();
                    let libraries = get_resource(&server.security.libraries, &self.profile)?
                        .filter_files(files.get(&Location::Libraries));
                    let assets =
                        get_resource(&server.security.assets, &profile_data.profile.assets)?
                            .filter_files(files.get(&Location::Assets));

                    let natives = get_resource(
                        &server.security.natives,
                        &NativeVersion {
                            version: profile_data.profile.version.clone(),
                            os_type: self.os_type.clone(),
                        },
                    )?;
                    let jre = get_resource(&server.security.jres, &self.os_type)?;
                    let profile = get_resource(&server.security.profiles, &self.profile)?
                        .filter_files(files.get(&Location::Profile));

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
        tx: UnboundedSender<Result<Message, warp::Error>>,
        server: Arc<RwLock<LaunchServer>>,
        client: &mut Client,
    ) {
        let server = server.read().await;
        send(tx, async {
            match server.profiles_data.get(&self.profile) {
                Some(profile_data) => {
                    let info = client.client_info.as_ref().unwrap();
                    let args = profile_data
                        .profile_info
                        .get_relevant_optionals(info, &self.optionals)
                        .map(Optional::get_args)
                        .flatten()
                        .collect::<Vec<_>>();
                    let mut profile = profile_data.profile.to_owned();
                    profile.jvm_args.extend(args);
                    Ok(ServerMessage::Profile(ProfileResponse { profile }))
                }
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
        tx: UnboundedSender<Result<Message, warp::Error>>,
        server: Arc<RwLock<LaunchServer>>,
        client: &mut Client,
    ) {
        let server = server.read().await;
        let info = client.client_info.as_ref().unwrap();
        let profiles_info: Vec<ProfileInfo> = server
            .profiles_data
            .values()
            .map(|data| {
                let mut data = data.profile_info.clone();
                data.retain_visible_optionals(info);
                data
            })
            .collect();
        send(tx, async {
            Ok(ServerMessage::ProfilesInfo(ProfilesInfoResponse {
                profiles_info,
            }))
        })
        .await;
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
        let ip = client.ip.clone();
        send(tx, async {
            let password = server.security.decrypt(&self.password)?;
            let result = server
                .auth_provider
                .auth(&self.login, &password, &ip)
                .await?;
            let access_token = SecurityManager::create_access_token();
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
        tx: UnboundedSender<Result<Message, warp::Error>>,
        server: Arc<RwLock<LaunchServer>>,
        _client: &mut Client,
    ) {
        let server = server.read().await;
        send(tx, async {
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
