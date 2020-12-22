use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;

use crate::server::auth::{has_join, join, HasJoinRequest};
use crate::server::websocket::ws_api;
use crate::LaunchServer;

mod auth;
pub mod profile;
mod websocket;

pub async fn start(data: Arc<RwLock<LaunchServer>>) -> std::io::Result<()> {
    let config = data.clone().read().await.config.clone();
    let data = warp::any().map(move || data.clone());
    let dir = warp::path("files").and(warp::fs::dir("static"));
    let client_ip = warp::header("x-real-ip")
        .or(warp::header("x-forwarded-for"))
        .unify()
        .or(warp::addr::remote().map(|addr: Option<SocketAddr>| addr.expect("Ip not found")))
        .unify();
    let ws = warp::path("api")
        .and(warp::ws())
        .and(data.clone())
        .and(client_ip)
        .map(|ws: warp::ws::Ws, launcher, addr: SocketAddr| {
            println!("remote address = {:?}", addr);
            ws.on_upgrade(move |socket| ws_api(socket, launcher))
        });
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
        .run(SocketAddr::from_str(&config.address).expect("Can't parse server address"))
        .await;
    Ok(())
}
