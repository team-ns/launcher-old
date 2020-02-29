mod websocket;
mod config;


#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    println!("Read config file...");
    let config = config::get_config()?;
    println!("Launch server starting...");
    websocket::start(&config).await
}

