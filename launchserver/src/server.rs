use crate::config::{Config, SecurityType};
use crate::server::http::has_join;
use crate::server::websocket::ws_api;
use crate::LauncherServiceProvider;
use anyhow::Result;
use ntex::web;
use ntex::web::middleware;
use ntex::web::types::Data;
use rustls::{internal::pemfile, NoClientAuth, ServerConfig};
use std::fs::File;
use std::io::BufReader;
use teloc::Resolver;

mod http;
mod websocket;

pub async fn run(data: Data<LauncherServiceProvider>) -> Result<()> {
    let config_sp = data.clone();
    let config: &Config = config_sp.resolve();

    let server = web::server(move || {
        web::App::new()
            .app_data(data.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/api").route(web::get().to(ws_api)))
            .service(ntex_files::Files::new("/files", "static"))
            .service(web::resource("/hasJoined").route(web::get().to(has_join)))
    })
    .workers(config.workers);
    match &config.security {
        SecurityType::Tls(tls_config) => {
            let mut server_config = ServerConfig::new(NoClientAuth::new());
            let cert_file = &mut BufReader::new(File::open(&tls_config.cert_file)?);
            let key_file = &mut BufReader::new(File::open(&tls_config.key_file)?);
            let cert_chain = pemfile::certs(cert_file).unwrap();
            let mut keys = pemfile::rsa_private_keys(key_file)
                .map_err(|_| anyhow::anyhow!("Can't parse pem rsa keys"))?;
            server_config.set_single_cert(cert_chain, keys.remove(0))?;
            server.bind_rustls(&config.bind_address, server_config)?
        }
        SecurityType::None => server.bind(&config.bind_address)?,
    }
    .run()
    .await?;
    Ok(())
}
