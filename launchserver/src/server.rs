use crate::config::Config;
use crate::server::http::has_join;
use crate::server::websocket::ws_api;
use crate::LauncherServiceProvider;
use anyhow::Result;
use ntex::web;
use ntex::web::middleware;
use ntex::web::types::Data;
use teloc::Resolver;

mod http;
mod websocket;

pub async fn run(data: Data<LauncherServiceProvider>) -> Result<()> {
    let config_sp = data.clone();
    let config: &Config = config_sp.resolve();
    web::server(move || {
        web::App::new()
            .app_data(data.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/api").route(web::get().to(ws_api)))
            .service(ntex_files::Files::new("/files", "static"))
            .service(web::resource("/hasJoined").route(web::get().to(has_join)))
    })
    .workers(config.workers)
    .bind(&config.bind_address)?
    .run()
    .await?;
    Ok(())
}
