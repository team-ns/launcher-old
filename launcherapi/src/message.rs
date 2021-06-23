use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::profile::{Profile, ProfileInfo};
use crate::validation::{ClientInfo, OsType, RemoteDirectory};

#[derive(Deserialize, Serialize, Debug)]
pub struct ClientRequest {
    pub request_id: Uuid,
    pub message: ClientMessage,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerResponse {
    pub request_id: Option<Uuid>,
    pub message: ServerMessage,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ClientMessage {
    Connected(ConnectedMessage),
    Auth(AuthMessage),
    JoinServer(JoinServerMessage),
    ProfileResources(ProfileResourcesMessage),
    Profile(ProfileMessage),
    ProfilesInfo(ProfilesInfoMessage),
    Custom(String),
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ServerMessage {
    Auth(AuthResponse),
    ProfileResources(ProfileResourcesResponse),
    Profile(ProfileResponse),
    ProfilesInfo(ProfilesInfoResponse),
    Runtime(String),
    Empty,
    Error(Error),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AuthMessage {
    pub login: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ConnectedMessage {
    pub client_info: ClientInfo,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JoinServerMessage {
    pub access_token: String,
    pub selected_profile: Uuid,
    pub server_id: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProfileResourcesMessage {
    pub profile: String,
    pub os_type: OsType,
    pub optionals: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProfileMessage {
    pub profile: String,
    pub optionals: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProfilesInfoMessage;

#[derive(Deserialize, Serialize, Debug)]
pub struct ProfileResponse {
    pub profile: Profile,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProfilesInfoResponse {
    pub profiles_info: Vec<ProfileInfo>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProfileResourcesResponse {
    pub profile: RemoteDirectory,
    pub libraries: RemoteDirectory,
    pub assets: RemoteDirectory,
    pub natives: RemoteDirectory,
    pub jre: RemoteDirectory,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub uuid: String,
    pub access_token: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Error {
    pub msg: String,
}
