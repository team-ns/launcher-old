use std::thread;
use log::info;
use shrust::{Shell, ShellIO};

use crate::config::Config;

pub fn start(config: Config) {
    let mut shell = Shell::new(config);
    register_commands(&mut shell);
    thread::spawn( move || shell.run_loop(&mut ShellIO::default()));
}

fn register_commands(shell: &mut Shell<Config>) {
    shell.new_command_noargs("build", "Build launcher", |io, v| {
        info!("Not working");
        Ok(())
    });
}