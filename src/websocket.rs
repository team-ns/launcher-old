use actix_web::{App, HttpServer, HttpResponse, web, HttpRequest, Error};
use actix::*;
use actix_files as fs;
use actix_web_actors::ws;
use std::time::{Instant, Duration};
use crate::config::Config;
use crate::websocket::message::Handle;

mod message;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

async fn api_route(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsApiSession {
            hb: Instant::now(),
        },
        &req,
        stream,
    )
}
/// websocket connection is long running connection, it easier
/// to handle with an actor
pub struct WsApiSession {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
}
impl Actor for WsApiSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsApiSession {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        // process websocket messages
        println!("WS: {:?}", msg);
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                let mes: Result<launcher_api::message::Message, serde_json::error::Error> = serde_json::from_str(&text);
                match mes {
                    Ok(m) => {m.handle( self, ctx)},
                    Err(e) => {println!("Error: {}", e)}
                }
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(_)) => {
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl WsApiSession {
    fn new() -> Self {
        Self { hb: Instant::now() }
    }

    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

pub async fn start(config: &Config) -> std::io::Result<()>{
    HttpServer::new(move || {
        App::new()
            // websocket
            .service(web::resource("/api/").to(api_route))
            // static resources
            .service(
                fs::Files::new("/static/", "static/")
                .show_files_listing()
                .use_last_modified(true)
            )
    })
        .bind(format!("{}:{}", config.address, config.port))?
        .run()
        .await
}
