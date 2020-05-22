use actix_files as fs;
use actix_web::App;
use actix_web::{web, HttpServer};

use crate::LaunchServer;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use std::sync::RwLock;

mod auth;
mod message;
pub mod profile;
mod websocket;

pub async fn start(data: Data<RwLock<LaunchServer>>) -> std::io::Result<()> {
    let config = data.read().unwrap().config.clone();
    HttpServer::new(move || {
        App::new()
            .data(data.clone())
            // server
            .service(web::resource("/api/").to(websocket::api_route))
            .service(web::resource("/join").route(web::post().to(auth::join)))
            .service(web::resource("/hasJoined").route(web::get().to(auth::has_join)))
            // static resources
            .service(
                fs::Files::new("/static", "static/")
                    .show_files_listing()
                    .use_last_modified(true),
            )
            .wrap(Logger::default())
    })
    .workers(config.workers)
    .bind(format!("{}:{}", config.address, config.port))?
    .run()
    .await
}
