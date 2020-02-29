use crate::websocket::WsApiSession;
use actix_web_actors::ws;
use actix_web_actors::ws::WebsocketContext;
use launcher_api::message::Message::Auth;
use launcher_api::message::{AuthMessage, Message};

type Context = ws::WebsocketContext<WsApiSession>;


pub trait Handle {
    fn handle(&self, client: &mut WsApiSession, ctx: &mut Context);
}

impl Handle for AuthMessage {
    fn handle(&self, client: &mut WsApiSession, ctx: &mut WebsocketContext<WsApiSession>) {
        ctx.text("Auth".to_string());
    }
}

impl Handle for Message {
    fn handle(&self, client: &mut WsApiSession, ctx: &mut Context) {
        match self {
            Auth(message) => { message.handle(client, ctx) }
            _ => {}
        }
    }
}
