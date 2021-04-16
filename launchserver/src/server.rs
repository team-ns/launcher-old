use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::http::StatusCode;
use warp::{Filter, Reply};

use crate::server::auth::{has_join, HasJoinRequest};
use crate::server::websocket::ws_api;
use crate::LaunchServer;

mod auth;
pub mod profile;
mod websocket;

pub async fn start(data: Arc<RwLock<LaunchServer>>) {
    let config = data.clone().read().await.config.clone();
    let data = warp::any().map(move || data.clone());
    let dir = warp::path("files")
        .and(warp::fs::dir("static"))
        .map(check_black_list);
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
            ws.on_upgrade(move |socket| ws_api(socket, launcher, format!("{:?}", addr.ip())))
        });
    let has_joined = warp::path("hasJoined")
        .and(warp::get())
        .and(warp::query::<HasJoinRequest>())
        .and(data.clone())
        .and_then(has_join);
    let routes = dir.or(ws).or(has_joined);
    warp::serve(routes)
        .run(SocketAddr::from_str(&config.bind_address).expect("Can't parse server address"))
        .await;
}

fn check_black_list(file: warp::filters::fs::File) -> warp::reply::Response {
    let mut iter = file.path().iter().skip(1);
    if let Some(os_str) = iter.next() {
        if Path::new(os_str).starts_with("profiles")
            && iter.last().map_or(false, |os_str| {
                let path: &Path = os_str.as_ref();
                profile::BLACK_LIST
                    .iter()
                    .any(|&file_name| path.ends_with(file_name))
            })
        {
            return StatusCode::NOT_FOUND.into_response();
        }
    }
    file.into_response()
}
