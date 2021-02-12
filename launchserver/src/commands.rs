use launcher_macro::{command, register_commands};
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
use futures::future::BoxFuture;
use futures::FutureExt;
use tokio::sync::RwLock;

use crate::server::profile;
use crate::LaunchServer;

type CmdFn = for<'a> fn(&'a mut LaunchServer, &'a [&str]) -> BoxFuture<'a, ()>;

struct Command {
    name: &'static str,
    description: &'static str,
    func: CmdFn,
}

#[derive(Hinter, Helper, Validator, Highlighter)]
struct CommandHelper {
    server: Arc<RwLock<LaunchServer>>,
    commands: HashMap<&'static str, &'static Command>,
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
        let selected_command = self.commands.get(&args[0]);
        match selected_command {
            None => println!("Command not found. Use help."),
            Some(&c) => {
                let args = &args[1..];
                let mut server = self.server.write().await;
                (c.func)(server.deref_mut(), args).await;
            }
        }
    }

    pub fn new_command(&mut self, command: &'static Command) {
        self.commands.insert(&command.name, command);
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
        register_commands!(rehash, sync);
        rl.set_helper(Some(helper));
        loop {
            let readline = rl.readline("");
            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str());
                    rl.helper_mut()
                        .unwrap()
                        .eval(line.trim_end_matches('\n').to_string())
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

#[command(description = "Update checksum of profile files")]
pub async fn rehash(server: &mut LaunchServer, args: &[&str]) {
    server.security.rehash(
        server.profiles.values(),
        args,
        server.config.file_server.clone(),
    );
}

#[command(description = "Sync profile list between server and client")]
pub async fn sync(server: &mut LaunchServer, _args: &[&str]) {
    let (profiles, profiles_info) = profile::get_profiles();
    server.profiles = profiles;
    server.profiles_info = profiles_info;
    info!("Sync was successfully finished!");
}
