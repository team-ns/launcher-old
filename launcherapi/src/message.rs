use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::profile::{Profile, ProfileInfo};
use crate::validation::{OsType, RemoteDirectory};

#[derive(Deserialize, Serialize)]
pub enum ClientMessage {
    Auth(AuthMessage),
    JoinServer(JoinServerMessage),
    ProfileResources(ProfileResourcesMessage),
    Profile(ProfileMessage),
    ProfilesInfo(ProfilesInfoMessage),
}

#[derive(Deserialize, Serialize)]
pub enum ServerMessage {
    Auth(AuthResponse),
    ProfileResources(ProfileResourcesResponse),
    Profile(ProfileResponse),
    ProfilesInfo(ProfilesInfoResponse),
    Empty,
    Error(Error),
}

#[derive(Deserialize, Serialize)]
pub struct AuthMessage {
    pub login: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct JoinServerMessage {
    pub access_token: String,
    pub selected_profile: Uuid,
    pub server_id: String,
}

#[derive(Deserialize, Serialize)]
pub struct ProfileResourcesMessage {
    pub profile: String,
    pub os_type: OsType,
    pub optionals: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct ProfileMessage {
    pub profile: String,
    pub optionals: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct ProfilesInfoMessage;

#[derive(Deserialize, Serialize)]
pub struct ProfileResponse {
    pub profile: Profile,
}

#[derive(Deserialize, Serialize)]
pub struct ProfilesInfoResponse {
    pub profiles_info: Vec<ProfileInfo>,
}

#[derive(Deserialize, Serialize)]
pub struct ProfileResourcesResponse {
    pub profile: RemoteDirectory,
    pub libraries: RemoteDirectory,
    pub assets: RemoteDirectory,
    pub natives: RemoteDirectory,
    pub jre: RemoteDirectory,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub uuid: String,
    pub access_token: String,
}

#[derive(Deserialize, Serialize)]
pub struct Error {
    pub msg: String,
}
