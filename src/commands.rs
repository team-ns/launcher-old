use shrust::{ShellIO, Shell};
use crate::config::Config;
use std::io::Write;

pub fn start(config: Config) {
    let mut shell = Shell::new(config);
    register_commands(&mut shell);
    shell.run_loop(&mut ShellIO::default());
}

fn register_commands(shell: &mut Shell<Config>) {
    shell.new_command_noargs("build", "Build launcher", |io, v| {
        writeln!(io, "Not working");
        Ok(())
    });
}