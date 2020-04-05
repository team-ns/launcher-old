use actix::prelude::Message as Msg;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Msg)]
#[rtype(result = "()")]
pub enum ClientMessage {
    Auth(AuthMessage),
    Profiles(ProfilesMessage),
    ProfileResources(ProfileResourcesMessage)
}

#[derive(Deserialize, Serialize, Msg)]
#[rtype(result = "()")]
pub enum ServerMessage {
    Auth(AuthResponse),
    ProfileResources(ProfileResourcesResponse),
    Error(Error)
}

#[derive(Deserialize, Serialize, Msg)]
#[rtype(result = "()")]
pub struct AuthMessage {
    pub login: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Msg)]
#[rtype(result = "()")]
pub struct ProfileResourcesMessage {
    pub profile: String
}

#[derive(Deserialize, Serialize, Msg)]
#[rtype(result = "()")]
pub struct ProfilesMessage {

}

#[derive(Deserialize, Serialize, Msg)]
#[rtype(result = "()")]
pub struct ProfileResourcesResponse {
    pub list: Vec<String>
}

#[derive(Deserialize, Serialize, Msg)]
#[serde(rename_all = "camelCase")]
#[rtype(result = "()")]
pub struct AuthResponse {
    pub uuid: String,
    pub access_token: String,
}

#[derive(Deserialize, Serialize, Msg)]
#[rtype(result = "()")]
pub struct Error {
    pub msg: String,
}