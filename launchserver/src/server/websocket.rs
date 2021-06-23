use std::sync::Arc;

use anyhow::Result;
use futures::SinkExt;
use launcher_api::message::{
    AuthMessage, AuthResponse, ClientMessage, ClientRequest, ConnectedMessage, Error,
    JoinServerMessage, ProfileMessage, ProfileResourcesMessage, ProfileResponse,
    ProfilesInfoMessage, ProfilesInfoResponse, ServerMessage, ServerResponse,
};
use launcher_api::optional::Optional;
use launcher_api::profile::ProfileInfo;
use launcher_extension_api::connection::Client;
use log::debug;
use log::error;
use ntex::web::{ws, HttpRequest, HttpResponse};
use ntex::{fn_factory_with_config, fn_service, map_config, rt, web, Service};
use std::cell::RefCell;
use std::io;
use std::ops::Deref;
use std::rc::Rc;
use teloc::Resolver;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{mpsc, RwLock};

use crate::auth::AuthProvider;
use crate::extensions::ExtensionService;
use crate::hash::resources::ProfileResources;
use crate::hash::HashingService;
use crate::profile::ProfileService;
use crate::security::SecurityService;
use crate::LauncherServiceProvider;

pub async fn ws_api(
    req: HttpRequest,
    pl: web::types::Payload,
    sp: web::types::Data<LauncherServiceProvider>,
) -> Result<HttpResponse, web::Error> {
    let address = req.connection_info().remote().map(String::from);
    let sp = sp.deref().clone();
    if let Some(address) = address {
        ws::start(
            req,
            pl,
            map_config(fn_factory_with_config(ws_service), move |cfg| {
                (cfg, address.to_string(), sp.clone())
            }),
        )
        .await
    } else {
        Ok(HttpResponse::BadRequest().finish())
    }
}

async fn ws_service(
    (sink, address, sp): (ws::WebSocketsSink, String, Arc<LauncherServiceProvider>),
) -> Result<
    impl Service<Request = ws::Frame, Response = Option<ws::Message>, Error = io::Error>,
    web::Error,
> {
    let (tx, rx) = mpsc::unbounded_channel();
    let client = Client::new(&address, tx);
    let extension_service: &ExtensionService = sp.resolve();
    extension_service.handle_connection(&client);
    let client = Rc::new(RefCell::new(client));
    rt::spawn(custom_messages(sink.clone(), rx));
    Ok(fn_service(move |frame| {
        let client = client.clone();
        let sp = sp.clone();
        async move {
            let result = match frame {
                ws::Frame::Ping(msg) => return Ok(Some(ws::Message::Pong(msg))),
                ws::Frame::Binary(body) => bincode::deserialize::<ClientRequest>(body.as_ref()),
                ws::Frame::Close(reason) => return Ok(Some(ws::Message::Close(reason))),
                _ => return Ok(Some(ws::Message::Close(None))),
            };
            if let Ok(msg) = result {
                debug!("Client message: {:?}", msg);
                let result = handle_message(msg.message, client.clone(), sp.clone()).await;
                let message = match result {
                    Ok(message) => message,
                    Err(e) => ServerMessage::Error(Error {
                        msg: format!("{}", e),
                    }),
                };
                let response = ServerResponse {
                    request_id: Some(msg.request_id),
                    message,
                };
                debug!("Response: {:?}", response);
                return match bincode::serialize(&response) {
                    Ok(bytes) => Ok(Some(ws::Message::Binary(bytes.into()))),
                    Err(e) => {
                        error!("Websocket send error: {}", e);
                        Ok(None)
                    }
                };
            }
            Ok(None)
        }
    }))
}

async fn custom_messages(mut sink: ws::WebSocketsSink, mut rx: UnboundedReceiver<ServerResponse>) {
    while let Some(msg) = rx.recv().await {
        let result = bincode::serialize(&msg);
        if let Ok(bytes) = result {
            if sink
                .send(Ok(ws::Message::Binary(bytes.into())))
                .await
                .is_err()
            {
                break;
            }
        }
    }
}

pub async fn handle_message(
    request: ClientMessage,
    client: Rc<RefCell<Client>>,
    sp: Arc<LauncherServiceProvider>,
) -> Result<ServerMessage> {
    let mut client = (*client).borrow_mut();
    let extension_service: &ExtensionService = sp.resolve();
    let msg = match extension_service.handle_message(&request, &mut client)? {
        None => match request {
            ClientMessage::Auth(auth) => auth.message_handle(sp, &mut client).await,
            ClientMessage::JoinServer(join) => join.message_handle(sp, &mut client).await,
            ClientMessage::Profile(profile) => profile.message_handle(sp, &mut client).await,
            ClientMessage::ProfileResources(resources) => {
                resources.message_handle(sp, &mut client).await
            }
            ClientMessage::ProfilesInfo(profiles_info) => {
                profiles_info.message_handle(sp, &mut client).await
            }
            ClientMessage::Connected(connected) => connected.message_handle(sp, &mut client).await,
            ClientMessage::Custom(body) => Ok(ServerMessage::Error(Error {
                msg: format!("No such extension that handles a custom message: {}", body),
            })),
        }?,
        Some(response) => response,
    };
    Ok(msg)
}

#[async_trait::async_trait]
pub trait MessageHandle {
    async fn message_handle(
        self,
        sp: Arc<LauncherServiceProvider>,
        client: &mut Client,
    ) -> Result<ServerMessage>;
}

#[async_trait::async_trait]
impl MessageHandle for ProfileResourcesMessage {
    async fn message_handle(
        self,
        sp: Arc<LauncherServiceProvider>,
        client: &mut Client,
    ) -> Result<ServerMessage> {
        let profile_service: Arc<RwLock<ProfileService>> = sp.resolve();
        let profile_service = profile_service.read().await;
        match profile_service.profiles_data.get(&self.profile) {
            Some(profile_data) => {
                let hashing_service: Arc<RwLock<HashingService>> = sp.resolve();
                let hashing_service = hashing_service.read().await;
                let info = client.client_info.as_ref().unwrap();
                hashing_service
                    .get_resources(info, profile_data, &self.optionals)
                    .map(ServerMessage::ProfileResources)
                    .map_err(|error| {
                        anyhow::anyhow!(
                            "Can't get resources, try synchronize profiles: {:?}",
                            error
                        )
                    })
            }
            None => Err(anyhow::anyhow!("This profile doesn't exist!")),
        }
    }
}

#[async_trait::async_trait]
impl MessageHandle for ProfileMessage {
    async fn message_handle(
        self,
        sp: Arc<LauncherServiceProvider>,
        client: &mut Client,
    ) -> Result<ServerMessage> {
        let profile_service: Arc<RwLock<ProfileService>> = sp.resolve();
        let profile_service = profile_service.read().await;
        match profile_service.profiles_data.get(&self.profile) {
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
    }
}

#[async_trait::async_trait]
impl MessageHandle for ProfilesInfoMessage {
    async fn message_handle(
        self,
        sp: Arc<LauncherServiceProvider>,
        client: &mut Client,
    ) -> Result<ServerMessage> {
        let profile_service: Arc<RwLock<ProfileService>> = sp.resolve();
        let profile_service = profile_service.read().await;
        let info = client.client_info.as_ref().unwrap();
        let profiles_info: Vec<ProfileInfo> = profile_service
            .profiles_data
            .values()
            .map(|data| {
                let mut data = data.profile_info.clone();
                data.retain_visible_optionals(info);
                data
            })
            .collect();
        Ok(ServerMessage::ProfilesInfo(ProfilesInfoResponse {
            profiles_info,
        }))
    }
}

#[async_trait::async_trait]
impl MessageHandle for ConnectedMessage {
    async fn message_handle(
        self,
        _sp: Arc<LauncherServiceProvider>,
        client: &mut Client,
    ) -> Result<ServerMessage> {
        client.client_info = Some(self.client_info);
        Ok(ServerMessage::Empty)
    }
}

#[async_trait::async_trait]
impl MessageHandle for AuthMessage {
    async fn message_handle(
        self,
        sp: Arc<LauncherServiceProvider>,
        client: &mut Client,
    ) -> Result<ServerMessage> {
        let security_service: &SecurityService = sp.resolve();
        let auth_provider: &AuthProvider = sp.resolve();
        let ip = client.ip.clone();
        let password = security_service.decrypt(&self.password)?;
        let result = auth_provider.auth(&self.login, &password, &ip).await?;
        let access_token = result.access_token;
        let uuid = result.uuid;
        client.username = Some(self.login.clone());
        client.access_token = Some(access_token.clone());
        Ok(ServerMessage::Auth(AuthResponse {
            uuid: uuid.to_string(),
            access_token,
        }))
    }
}

#[async_trait::async_trait]
impl MessageHandle for JoinServerMessage {
    async fn message_handle(
        self,
        sp: Arc<LauncherServiceProvider>,
        _client: &mut Client,
    ) -> Result<ServerMessage> {
        let auth_provider: &AuthProvider = sp.resolve();
        let e = auth_provider.get_entry(&self.selected_profile).await?;
        if e.access_token.is_some() && e.access_token.unwrap().eq(&self.access_token) {
            auth_provider
                .update_server_id(&self.selected_profile, &self.server_id)
                .await?;
            Ok(ServerMessage::Empty)
        } else {
            Ok(ServerMessage::Error(Error {
                msg: String::from("Access token error"),
            }))
        }
    }
}
