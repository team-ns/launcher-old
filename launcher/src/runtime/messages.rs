use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum RuntimeMessage {
    Login { login: String, password: String },
    Play { profile: String },
}
