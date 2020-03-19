use gtk::prelude::*;
use gtk::{ApplicationWindow, Builder, Button};
use relm_derive::Msg;
use relm::{Relm, Update, Widget};

use crate::client::ClientHandler;
use crate::client;

mod resources;

#[derive(Msg)]
pub enum Msg {
    Login,
}

pub struct Model {
    client_handler: ClientHandler,
}

pub struct Runtime {
    model: Model,
    window: ApplicationWindow,
}


impl Update for Runtime {
    type Model = Model;
    type ModelParam = ();
    type Msg = Msg;

    fn model(_: &Relm<Self>, _: ()) -> Model {
        Model {
            client_handler: client::ClientHandler::new("ws://localhost:9090/api/"),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Login =>  self.model.client_handler.auth("Test", "test"),
        }
    }
}

impl Widget for Runtime {
    type Root = ApplicationWindow;

    fn root(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let provider = gtk::CssProvider::new();
        provider
            .load_from_data(include_str!("../runtime/style.css").as_ref())
            .expect("Failed to load CSS");

        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let glade_src = include_str!("../runtime/main.glade");

        let builder = Builder::new_from_string(glade_src);
        let button: Button = builder.get_object("login").unwrap();
        let window: ApplicationWindow = builder.get_object("window").unwrap();

        relm::connect!(relm, button, connect_clicked(_), Msg::Login);

        window.show_all();

        Runtime {
            model,
            window,
        }
    }
}