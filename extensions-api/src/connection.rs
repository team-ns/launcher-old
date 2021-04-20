use crate::launcher::message::ServerResponse;
use crate::launcher::validation::ClientInfo;
pub use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

pub struct Client {
    pub uuid: Uuid,
    pub ip: String,
    pub access_token: Option<String>,
    pub username: Option<String>,
    pub client_info: Option<ClientInfo>,
    pub channel: UnboundedSender<ServerResponse>,
}

impl Client {
    pub fn new(ip: &str, tx: UnboundedSender<ServerResponse>) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            ip: ip.to_string(),
            access_token: None,
            username: None,
            client_info: None,
            channel: tx,
        }
    }
}
