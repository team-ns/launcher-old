use rust_embed::RustEmbed;

use crate::client::WebSocketClient;
use sciter::{Element, HELEMENT};
use sciter::dom::event::{BEHAVIOR_EVENTS, PHASE_MASK};
use sciter::dom::EventReason;

mod resources;


#[derive(RustEmbed)]
#[folder = "runtime/"]
struct Asset;


struct Handler(WebSocketClient);

impl sciter::EventHandler for Handler {
    fn on_event(&mut self, root: HELEMENT, source: HELEMENT, target: HELEMENT, code: BEHAVIOR_EVENTS, phase: PHASE_MASK, reason: EventReason) -> bool {
        let source = Element::from(source);
        if phase == PHASE_MASK::SINKING && code == BEHAVIOR_EVENTS::BUTTON_CLICK && source.get_attribute("id").unwrap_or("none".to_string()).eq(&"login"){
            let root = Element::from(root).root();
            let username = root.find_first("#username").unwrap().expect("div#message not found").get_value().as_string().unwrap();
            let password = root.find_first("#password").unwrap().expect("div#message not found").get_value().as_string().unwrap();
            self.0.auth(&username, &password);
            return true;
        }
        return false;
    }
}

pub fn start(client: WebSocketClient) {
    let html = Asset::get("minimal.htm").unwrap().to_mut().clone();

    sciter::set_options(sciter::RuntimeOptions::ScriptFeatures(
        sciter::SCRIPT_RUNTIME_FEATURES::ALLOW_SYSINFO as u8        // Enables `Sciter.machineName()`
            | sciter::SCRIPT_RUNTIME_FEATURES::ALLOW_FILE_IO as u8    // Enables opening file dialog (`view.selectFile()`)
    )).unwrap();

    sciter::set_options(sciter::RuntimeOptions::DebugMode(true)).unwrap();

    let mut frame = sciter::window::Builder::main_window()
        .with_size((800, 600))
        .fixed()
        .create();

    frame.load_html(&html, Some("example://minimal.htm"));

    frame.event_handler(Handler(client));

    frame.run_app();

}

