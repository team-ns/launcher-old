#![windows_subsystem = "windows"]

use std::error::Error;

mod client;
mod config;
mod game;
mod runtime;
mod security;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    runtime::run();
    Ok(())
}
