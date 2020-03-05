mod game;

fn main() {
    game::Client::start(&game::Client{name: String::from("test")});
}


