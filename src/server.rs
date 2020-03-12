use std::borrow::Borrow;

use actix_files as fs;
use actix_web::{HttpServer, web};
use actix_web::App;

use crate::config::Config;
use actix_web::middleware::Logger;

mod message;
mod auth;
mod websocket;

pub async fn start(config: Config) -> std::io::Result<()> {
    let data = config.borrow().clone();
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
                    .use_last_modified(true)
            )
            .wrap(Logger::default())

    })
        .workers(config.workers)
        .bind(format!("{}:{}", config.address, config.port))?
        .run()
        .await
}
