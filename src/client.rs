use tokio_tungstenite::{connect_async, WebSocketStream};
use futures_util::{StreamExt, SinkExt};
use futures_util::stream::SplitSink;
use tungstenite::protocol::Message::Text;
use tungstenite::protocol::Message;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use launcher_api::message::{ClientMessage, AuthMessage, ServerMessage};

use crate::security::SecurityManager;
use crate::security;

pub struct ClientHandler {
    writer: SplitSink<WebSocketStream<TcpStream>, Message>,
    runtime: Runtime,
    security: SecurityManager,
}

impl ClientHandler {
    pub fn new(address: &str) -> Self {
        let mut rt = Runtime::new().unwrap();
        let socket = rt.block_on(connect_async(address)).unwrap().0;

        let (write, read) = socket.split();
        rt.spawn(async {
            use tokio::runtime::Handle;

            let handle = Handle::current();
            handle.spawn(
                read.for_each(|message| async {
                handle_message(message.unwrap());
            }));
        });

        ClientHandler { writer: write, runtime: rt, security: security::get_manager()}
    }


    pub fn auth(&mut self, login: &str, password: &str) {
        let message = ClientMessage::Auth(
            AuthMessage {
                login: String::from(login),
                password: self.security.encrypt(password)
            }
        );
        self.send_message(message);
    }

    fn send_message(&mut self, message: ClientMessage) {
        self.runtime.block_on(
            self.writer.send(Message::Text(serde_json::to_string(&message).unwrap()))
        ).unwrap();
    }
}

fn handle_message(message: Message) {
    if message.is_text() {
        let sever_msg: ServerMessage = serde_json::from_str(&message.into_text().unwrap()).unwrap();
        match sever_msg {
            ServerMessage::Auth(auth) => {
                println!("UUID - {} : TOKEN - {}", auth.uuid, auth.access_token);
            }
            _ => {}
        }
    }
}