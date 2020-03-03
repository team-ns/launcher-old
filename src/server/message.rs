use actix::{AsyncContext, Handler};
use launcher_api::message::{AuthMessage, Message};
use launcher_api::message::Message::Auth;

use crate::config::auth::{AuthResult, Error};
use crate::server::websocket::WsApiSession;

impl Handler<AuthResult> for WsApiSession {
    type Result = ();
    //TODO: Add generating access token and send it with UUID
    fn handle(&mut self, msg: AuthResult, ctx: &mut Self::Context) -> Self::Result {
        if msg.message.is_none() {
            ctx.text("You auth".to_string());
        } else {
            ctx.text(format!("Error: {}", msg.message.unwrap()));
        }
    }
}

impl Handler<Error> for WsApiSession {
    type Result = ();

    fn handle(&mut self, msg: Error, ctx: &mut Self::Context) -> Self::Result {
       ctx.text(format!("Error: {}", msg.message));
    }
}

impl Handler<AuthMessage> for WsApiSession {
    type Result = ();

    fn handle(&mut self, msg: AuthMessage, ctx: &mut Self::Context) -> Self::Result {
        let auth = self.config.auth.get_provide();
        let ip = self.ip.clone();
        let addr = ctx.address();
        ctx.spawn(actix::fut::wrap_future(async move {
            let result = auth.auth(&msg.login, &msg.password, &ip).await;
            match result {
                Ok(auth_result) => {
                    addr.do_send(auth_result);
                }

                Err(e) => {
                    addr.do_send(e);
                }
            }
        }));
    }
}

impl Handler<Message> for WsApiSession {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            Auth(message) => {
                ctx.address().do_send(message);
            }
            _ => {}
        }
    }
}
