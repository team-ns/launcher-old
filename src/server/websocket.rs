use std::time::{Duration, Instant};
use log::error;
use crate::LaunchServer;
use futures::StreamExt;
use launcher_api::message::ClientMessage;
use serde::export::Result::Ok;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::filters::ws::WebSocket;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn ws_api(ws: WebSocket, server: Arc<RwLock<LaunchServer>>) {
    let (tx, mut rx) = ws.split();
    while let Some(result) = rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("Websocket error");
                break;
            }
        };
        if let Ok(json) = msg.to_str() {
            if let Ok(message) = serde_json::from_str::<ClientMessage>(json) {
                match message {
                    ClientMessage::Auth(auth) => {}
                    ClientMessage::Profiles(profiles) => {}
                    ClientMessage::ProfileResources(resources) => {}
                }
            }
        }
    }
}
