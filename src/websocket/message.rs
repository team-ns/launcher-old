use crate::websocket::WsApiSession;
use actix_web_actors::ws;
use actix_web_actors::ws::WebsocketContext;
use serde::{Deserialize, Serialize};
use crate::websocket::message::Message::Auth;

type Context = ws::WebsocketContext<WsApiSession>;

#[derive(Deserialize, Serialize)]
pub enum Message {
    Auth(AuthMessage),
    Profiles(ProfilesMessage)
}
#[derive(Deserialize, Serialize)]
pub struct AuthMessage {

}
#[derive(Deserialize, Serialize)]
pub struct ProfilesMessage {

}
trait Handle {
    fn handle(&self, client: &mut WsApiSession, ctx: &mut Context);
}

impl Handle for AuthMessage {
    fn handle(&self, client: &mut WsApiSession, ctx: &mut WebsocketContext<WsApiSession>) {
        ctx.text("Auth".to_string());
    }
}
 impl Message {
    pub fn handle(&self, client: &mut WsApiSession, ctx: &mut Context) {
        match self {
            Auth(message) => {message.handle(client, ctx)}
            _ => {}
        }
    }
}