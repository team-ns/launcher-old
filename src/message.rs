use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum Message {
    Auth(AuthMessage),
    Profiles(ProfilesMessage)
}
#[derive(Deserialize, Serialize)]
pub struct AuthMessage {
    pub login: String,
    pub password: String,
}
#[derive(Deserialize, Serialize)]
pub struct ProfilesMessage {

}