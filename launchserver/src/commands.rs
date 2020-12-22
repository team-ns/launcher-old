use log::info;
use rustyline::completion::{extract_word, Completer};
use rustyline::error::ReadlineError;
use rustyline::Config as LineConfig;
use rustyline::{CompletionType, Context, EditMode, Editor, OutputStreamType};
use rustyline_derive::{Helper, Highlighter, Hinter, Validator};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::server::profile;
use crate::LaunchServer;

type CmdFn = Box<dyn Fn(&mut LaunchServer, &[&str]) -> () + Send + Sync>;

struct Command {
    name: String,
    description: String,
    func: CmdFn,
}

impl Command {
    fn new(name: &str, description: &str, command: CmdFn) -> Self {
        Command {
            name: name.to_string(),
            description: description.to_string(),
            func: command,
        }
    }
}

#[derive(Hinter, Helper, Validator, Highlighter)]
struct CommandHelper {
    server: Arc<RwLock<LaunchServer>>,
    commands: HashMap<String, Command>,
}

impl Completer for CommandHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let (word_start, _) = extract_word(line, pos, None, &[32u8][..]);
        let results = self
            .commands
            .keys()
            .filter_map(|cmd| {
                if cmd.starts_with(line.trim()) {
                    Some(cmd.to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok((word_start, results))
    }
}

impl CommandHelper {
    pub fn new(server: Arc<RwLock<LaunchServer>>) -> Self {
        CommandHelper {
            server,
            commands: HashMap::new(),
        }
    }

    pub async fn eval(&mut self, command: String) {
        let args: Vec<&str> = command.split(' ').collect();
        if args[0].eq("help") {
            for command in self.commands.values() {
                println!("{} - {}", command.name, command.description);
            }
            return;
        }
        let selected_command = self.commands.get(&args[0].to_string());
        match selected_command {
            None => println!("Command not found. Use help."),
            Some(c) => {
                let args = &args[1..];
                let mut server = self.server.write().await;
                (c.func)(server.deref_mut(), args);
            }
        }
    }

    pub fn new_command<F>(&mut self, name: &str, description: &str, command: F)
    where
        F: Fn(&mut LaunchServer, &[&str]) + Send + Sync + 'static,
    {
        self.commands.insert(
            name.to_string(),
            Command::new(name, description, Box::new(command)),
        );
    }
}

pub async fn start(server: Arc<RwLock<LaunchServer>>) {
    tokio::spawn(async move {
        let rl_config = LineConfig::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .output_stream(OutputStreamType::Stdout)
            .build();
        let mut rl: Editor<CommandHelper> = Editor::with_config(rl_config);
        let mut helper = CommandHelper::new(server);
        register_command(&mut helper);
        rl.set_helper(Some(helper));
        loop {
            let readline = rl.readline("");
            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str());
                    rl.helper_mut()
                        .unwrap()
                        .eval(line.trim_end_matches("\n").to_string())
                        .await;
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Bye");
                    exit(0);
                }
                _ => {}
            }
        }
    });
}

fn register_command(helper: &mut CommandHelper) {
    helper.new_command("rehash", "Update checksum of profile files", rehash);
    helper.new_command("sync", "Sync profile list between server and client", sync);
}

pub fn rehash(server: &mut LaunchServer, args: &[&str]) {
    server.security.rehash(server.profiles.values(), args);
}

pub fn sync(server: &mut LaunchServer, _args: &[&str]) {
    let (profiles, profiles_info) = profile::get_profiles();
    server.profiles = profiles;
    server.profiles_info = profiles_info;
    info!("Sync was successfully finished!");
}
