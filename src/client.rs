use std::sync::mpsc::{channel, Sender};
use std::sync::mpsc::Receiver;
use std::thread;

use launcher_api::message::{AuthMessage, ClientMessage, ServerMessage};
use launcher_api::message::Error;
use launcher_api::message::ServerMessage::{Auth, Error as OtherError};
use openssl::ssl::{SslConnector, SslMethod, SslStream};
use ws::{ErrorKind, Handler, Handshake, Request, WebSocket};
use ws::Message;
use ws::util::TcpStream;

use crate::security;
use crate::security::SecurityManager;
use url::Url;

pub struct WebSocketClient {
    out: ws::Sender,
    recv: Receiver<ServerMessage>,
    security: SecurityManager,
}

pub struct Client {
    out: ws::Sender,
    sender: Sender<ServerMessage>
}

impl Handler for Client {

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        match msg {
            Message::Text(t) => {
                if let Ok(message) = serde_json::from_str::<ServerMessage>(&t) {
                    self.sender.send(message);
                } else {
                    self.sender.send(ServerMessage::Error(Error { msg: "work".to_string() }));
                }
            }
            Message::Binary(_) => {}
        }
        Ok(())
    }

    fn build_request(&mut self, url: &Url) -> ws::Result<Request> {
        let mut req = Request::from_url(url)?;
        req.headers_mut()
            .push(("Origin".into(), get_origin(url).into()));
        Ok(req)
    }




    /* fn upgrade_ssl_client(
         &mut self,
         sock: TcpStream,
         _: &url::Url,
     ) -> ws::Result<SslStream<TcpStream>> {
         let mut builder = SslConnector::builder(SslMethod::tls()).map_err(|e| {
             ws::Error::new(
                 ws::ErrorKind::Internal,
                 format!("Failed to upgrade client to SSL: {}", e),
             )
         })?;
         builder.set_verify(SslVerifyMode::empty());

         let connector = builder.build();
         connector
             .configure()
             .unwrap()
             .use_server_name_indication(false)
             .verify_hostname(false)
             .connect("", sock)
             .map_err(From::from)
     }*/
}


impl WebSocketClient {
    pub fn new(address: &str) -> Self {
        let ( s, r) = channel::<ServerMessage>();
        let mut ws = WebSocket::new(move |out|
            Client {
                sender: s.clone(),
                out
            }
        ).unwrap();
        let parsed: url::Url = Url::parse(&address.to_string()).unwrap();
        ws.connect(parsed);
        let sender = ws.broadcaster();

        thread::Builder::new()
            .name("websocket_handler".to_owned())
            .spawn(move || {
                // This blocks the thread
                ws.run();
            })
            .expect("Failed to start WebSocket thread");
        WebSocketClient {
            security: security::get_manager(),
            recv: r,
            out: sender.clone(),
        }
    }


    pub fn auth(&mut self, login: &str, password: &str) {
        let message = ClientMessage::Auth(
            AuthMessage {
                login: String::from(login),
                password: self.security.encrypt(password),
            }
        );
        match self.send_sync(message) {
            Auth(auth) => {
                println!("UUID - {} : TOKEN - {}", auth.uuid, auth.access_token);
            }
            OtherError(e) => { println!("Error: {}", e.msg) }
        };
    }


    fn send_sync(&mut self, msg: ClientMessage) -> ServerMessage {
        self.out.send(Message::text(serde_json::to_string(&msg).unwrap()));
        match self.recv.recv() {
            Ok(message) => {
                message
            }
            Err(e) => {
                ServerMessage::Error(Error { msg: "what".to_string() })
            }
        }

    }
}

fn get_origin(url: &Url) -> String {
    let scheme = if url.scheme() == "wss" {
        "https"
    } else {
        "http"
    };

    format!("{}://{}", scheme, url.host_str().unwrap_or(""))
}
