use serde::{Deserialize, Serialize};

use crate::profile::{Profile, ProfileInfo};
use crate::validation::{HashedDirectory, OsType};

#[derive(Deserialize, Serialize)]
pub enum ClientMessage {
    Auth(AuthMessage),
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
    Error(Error),
}

#[derive(Deserialize, Serialize)]
pub struct AuthMessage {
    pub login: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct ProfileResourcesMessage {
    pub profile: String,
    pub os_type: OsType,
}

#[derive(Deserialize, Serialize)]
pub struct ProfileMessage {
    pub profile: String,
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
    pub profile: HashedDirectory,
    pub libraries: HashedDirectory,
    pub assets: HashedDirectory,
    pub natives: HashedDirectory,
    pub jre: HashedDirectory,
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
