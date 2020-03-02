use serde::{Deserialize, Serialize};
use actix::prelude::Message as Msg;

#[derive(Deserialize, Serialize, Msg)]
#[rtype(result = "()")]
pub enum Message {
    Auth(AuthMessage),
    Profiles(ProfilesMessage)
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