use std::sync::Arc;

use futures::TryFutureExt;
use futures::{FutureExt, StreamExt};
use tokio::sync::RwLock;
use warp::Filter;

use crate::server::auth::{has_join, join, HasJoinRequest};
use crate::server::websocket::ws_api;
use crate::LaunchServer;
use std::net::SocketAddr;
use std::str::FromStr;

mod auth;
pub mod profile;
mod websocket;

pub async fn start(data: Arc<RwLock<LaunchServer>>) -> std::io::Result<()> {
    let config = data.read().await.config.clone();
    let data = warp::any().map(move || data.clone());
    let dir = warp::path("files").and(warp::fs::dir("static"));
    let ws = warp::path("api")
        .and(warp::ws())
        .and(data.clone())
        .map(|ws: warp::ws::Ws, launcher| ws.on_upgrade(move |socket| ws_api(socket, launcher)));
    let join = warp::path("join")
        .and(warp::post())
        .and(warp::body::json())
        .and(data.clone())
        .and_then(join);
    let has_joined = warp::path("hasJoined")
        .and(warp::get())
        .and(warp::query::<HasJoinRequest>())
        .and(data.clone())
        .and_then(has_join);
    let routes = dir.or(ws).or(join).or(has_joined);
    warp::serve(routes)
        .run(SocketAddr::from_str(&config.address).unwrap())
        .await;
    Ok(())
}
