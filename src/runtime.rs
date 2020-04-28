use rust_embed::RustEmbed;
use sciter::{Element, HELEMENT, Value};
use sciter::{dispatch_script_call, vmap};
use sciter::dom::event::{BEHAVIOR_EVENTS, PHASE_MASK};
use sciter::dom::EventReason;

use crate::client::WebSocketClient;

mod resources;


#[derive(RustEmbed)]
#[folder = "runtime/"]
struct Asset;


struct Handler {
    ws: WebSocketClient,
    root: Option<Element>
}

impl sciter::EventHandler for Handler {
    fn document_complete(&mut self, root: HELEMENT, target: HELEMENT) {
        let element = Element::from(root);
        self.root = Some(element);
    }

   /* dispatch_script_call! {
		fn load_profiles();
		fn login();
	}
*/

}

impl Handler {
     /*fn load_profiles(&mut self) -> Value {
        //TODO: Add websocket profiles
        let data = vmap! {
                  "name" => "Test",
                  "value" => "testik",
                 };
        let mut value = Value::array(0);
        value.push(data);
        let data = vmap! {
                  "name" => "test2",
                  "value" => "testik",
                 };
        value.push(data);
        value
    }

    fn login(&mut self) -> Value {
        let root = self.root.as_ref().unwrap();
        let username = root.find_first("#username")
            .unwrap()
            .expect("username not found")
            .get_value()
            .as_string()
            .unwrap();
        let password = root.find_first("#password")
            .unwrap()
            .expect("password not found")
            .get_value()
            .as_string()
            .unwrap();
        let res = self.ws.auth(&username, &password);
        //TODO: Add remember check
        Value::from(res.is_ok())
    }*/
}

pub fn start(client: WebSocketClient) {
    let resources = Asset::get("app.rc").unwrap().to_mut().clone();

    sciter::set_options(sciter::RuntimeOptions::ScriptFeatures(
        sciter::SCRIPT_RUNTIME_FEATURES::ALLOW_SYSINFO as u8
            | sciter::SCRIPT_RUNTIME_FEATURES::ALLOW_FILE_IO as u8
            | sciter::SCRIPT_RUNTIME_FEATURES::ALLOW_EVAL as u8
    )).unwrap();

    sciter::set_options(sciter::RuntimeOptions::DebugMode(true)).unwrap();

    let mut frame = sciter::window::Builder::main_window()
        .with_size((800, 600))
        .fixed()
        .create();


    frame.archive_handler(&resources).expect("Invalid archive");

    frame.event_handler(Handler {
        ws: client,
        root: None
    });

    frame.load_file("this://app/menu.htm");
    frame.run_app();
}

