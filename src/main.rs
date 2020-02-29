mod websocket;
mod config;


#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let config = config::get_config()?;
    websocket::start(&config).await
}

