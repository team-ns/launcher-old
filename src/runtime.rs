use rust_embed::RustEmbed;
use sciter::{Element, HELEMENT, Value};
use sciter::dom::event::{BEHAVIOR_EVENTS, PHASE_MASK};
use sciter::dom::EventReason;
use sciter::vmap;
use crate::client::WebSocketClient;

mod resources;


#[derive(RustEmbed)]
#[folder = "runtime/"]
struct Asset;


struct Handler {
    ws: WebSocketClient,
}

impl sciter::EventHandler for Handler {
    fn on_event(&mut self, root: HELEMENT, source: HELEMENT, target: HELEMENT, code: BEHAVIOR_EVENTS, phase: PHASE_MASK, reason: EventReason) -> bool {
        if phase != PHASE_MASK::BUBBLING {
            return false;
        }
        let source = Element::from(source);
        if code == BEHAVIOR_EVENTS::BUTTON_CLICK &&
            source.get_attribute("id")
                .unwrap_or("none".to_string())
                .eq(&"login") {
            let root = Element::from(root).root();
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
            if res.is_ok() {
                let res = res.ok().unwrap();
                let data = vmap! {
                  "uuid" => res.uuid,
                  "accessToken" => res.access_token,
                 };
                source.fire_event(
                    BEHAVIOR_EVENTS::CUSTOM,
                    None,
                    Some(source.as_ptr()),
                    false,
                    Some(data)
                ).expect("Failed to fire event");
            }

            return true;
        }
        return false;
    }
}

pub fn start(client: WebSocketClient) {
    let resources = Asset::get("app.rc").unwrap().to_mut().clone();

    sciter::set_options(sciter::RuntimeOptions::ScriptFeatures(
        sciter::SCRIPT_RUNTIME_FEATURES::ALLOW_SYSINFO as u8
            | sciter::SCRIPT_RUNTIME_FEATURES::ALLOW_FILE_IO as u8
    )).unwrap();

    sciter::set_options(sciter::RuntimeOptions::DebugMode(true)).unwrap();

    let mut frame = sciter::window::Builder::main_window()
        .with_size((800, 600))
        .fixed()
        .create();

    frame.archive_handler(&resources).expect("Invalid archive");

    frame.load_file("this://app/index.htm");

    frame.event_handler(Handler {
        ws: client
    });

    frame.run_app();
}

