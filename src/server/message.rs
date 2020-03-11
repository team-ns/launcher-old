use actix::{AsyncContext, Handler, WrapFuture, ContextFutureSpawner};
use launcher_api::message::{AuthMessage, ClientMessage, ServerMessage, AuthResponse};
use launcher_api::message::ClientMessage::Auth;
use rand::Rng;
use actix::fut::wrap_future;
use futures::TryFutureExt;

use crate::config::auth::{AuthResult, Error};
use crate::server::websocket::WsApiSession;

impl Handler<AuthResult> for WsApiSession {
    type Result = ();
    fn handle(&mut self, msg: AuthResult, ctx: &mut Self::Context) -> Self::Result {
        if msg.message.is_none() {
            let mut rng = rand::thread_rng();
            let digest = md5::compute(format!("{}{}{}", rng.gen_range(1000000000, 2147483647), rng.gen_range(1000000000, 2147483647), rng.gen_range(0, 9)));
            let access_token = format!("{:x}", digest);
            let auth = self.config.auth.get_provide();
            let uuid = msg.uuid.unwrap();
            ctx.text(serde_json::to_string(&ServerMessage::Auth(AuthResponse { uuid: uuid.to_string(), access_token: access_token.to_string() })).unwrap());
            ctx.spawn(actix::fut::wrap_future(async move { auth.update_access_token(&uuid, &access_token.clone()).await; }));
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
        let password = self.config.security.decrypt(&msg.password);
        ctx.spawn(actix::fut::wrap_future(async move {
            let result = auth.auth(&msg.login, &password, &ip).await;
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

impl Handler<ClientMessage> for WsApiSession {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            Auth(message) => {
                ctx.address().do_send(message);
            }
            _ => {}
        }
    }
}
