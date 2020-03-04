use actix::prelude::Message as Msg;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Msg)]
#[rtype(result = "()")]
pub enum ClientMessage {
    Auth(AuthMessage),
    Profiles(ProfilesMessage)
}

#[derive(Deserialize, Serialize, Msg)]
#[rtype(result = "()")]
pub enum ServerMessage {
    Auth(AuthResponse),
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
pub struct ProfilesMessage {

}

#[derive(Deserialize, Serialize, Msg)]
#[serde(rename_all = "camelCase")]
#[rtype(result = "()")]
pub struct AuthResponse {
    pub uuid: String,
    pub access_token: String,
}

pub struct Error {
    pub msg: String,
}