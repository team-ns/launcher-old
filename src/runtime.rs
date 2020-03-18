use gio::prelude::*;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Builder, Button, MessageDialog, CssProvider};

pub fn start() {
    let application = gtk::Application::new(
        Some("nslauncher"),
        Default::default(),
    ).expect("Initialization failed...");

    let provider = gtk::CssProvider::new();
    provider
        .load_from_data(include_str!("../runtime/style.css").as_ref())
        .expect("Failed to load CSS");

    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    application.connect_activate(|app| {
        create_ui(app);
    });

    application.run(&[]);
}

fn create_ui(application: &gtk::Application) {
    let glade_src = include_str!("../runtime/main.glade");
    let builder = Builder::new_from_string(glade_src);

    builder.connect_signals(move |_, handler_name| {
        if handler_name == "login" {
            Box::new(move |_| {
                login();
                None
            })
        } else {
            panic!("Unknown handler name {}", handler_name)
        }
    });
    let window: ApplicationWindow = builder.get_object("window").unwrap();
    window.set_application(Some(application));
    window.show_all();
}

fn login() {
    println!("login");
}