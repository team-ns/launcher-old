use futures::future::BoxFuture;
use futures::FutureExt;
use launcher_extension_api::command::ExtensionCommand;
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
use tokio::sync::RwLock;

use crate::security::SecurityManager;
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
    extension_commands: HashMap<String, HashMap<String, ExtensionCommand>>,
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
    pub async fn new(server: Arc<RwLock<LaunchServer>>) -> Self {
        let server_read = server.read().await;
        let extension_manager = &server_read.extension_manager;
        CommandHelper {
            server: server.clone(),
            commands: HashMap::new(),
            extension_commands: extension_manager.get_commands(),
        }
    }

    pub async fn eval(&mut self, command: String) {
        let args: Vec<&str> = command.split(' ').map(str::trim).collect();
        if args[0].eq("help") {
            for command in self.commands.values() {
                println!("[launcher] {} - {}", command.name, command.description);
            }
            for (extension, commands) in &self.extension_commands {
                for (name, command) in commands {
                    println!("[{}] {} - {}", extension, name, command.description);
                }
            }
            return;
        }
        let selected_command = self.commands.get(&args[0]);
        match selected_command {
            None => {
                let command = self
                    .extension_commands
                    .values()
                    .filter_map(|commands| commands.get(&args[0].to_string()))
                    .last();
                if let Some(c) = command {
                    let args = &args[1..];
                    c.executor.execute(args);
                } else {
                    println!("Command not found. Use help.")
                }
            }
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
        let mut helper = CommandHelper::new(server).await;
        register_commands!(rehash, sync, auth);
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

#[command(description = "Authorize account with provided login and password")]
pub async fn auth(server: &mut LaunchServer, args: &[&str]) {
    if args.len() < 2 {
        info!("Expected correct arguments number. Use auth <login> <password>")
    } else {
        match server
            .auth_provider
            .auth(args[0], args[1], "127.0.0.1")
            .await
        {
            Ok(uuid) => {
                let access_token = SecurityManager::create_access_token();
                match server
                    .auth_provider
                    .update_access_token(&uuid, &access_token)
                    .await
                {
                    Ok(_) => info!(
                        "Success auth: login '{}', uuid '{}', access_token '{}'",
                        args[0], uuid, access_token
                    ),
                    Err(error) => info!("Failed to update access_token: {}", error),
                }
            }
            Err(error) => info!("Failed to auth: {}", error),
        }
    }
}
