use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum ClientMessage {
    Auth(AuthMessage),
    Profiles(ProfilesMessage),
    ProfileResources(ProfileResourcesMessage)
}

#[derive(Deserialize, Serialize)]
pub enum ServerMessage {
    Auth(AuthResponse),
    ProfileResources(ProfileResourcesResponse),
    Error(Error)
}

#[derive(Deserialize, Serialize)]
pub struct AuthMessage {
    pub login: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct ProfileResourcesMessage {
    pub profile: String
}

#[derive(Deserialize, Serialize)]
pub struct ProfilesMessage {

}

#[derive(Deserialize, Serialize)]
pub struct ProfileResourcesResponse {
    pub list: Vec<String>
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