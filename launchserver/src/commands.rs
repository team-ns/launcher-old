use crate::auth::AuthProvider;
use crate::config::Config;
use crate::extensions::ExtensionService;
use crate::profile::ProfileService;
use crate::security::SecurityService;
use crate::{hash, profile, LauncherServiceProvider};
use anyhow::Result;
use ecies_ed25519::PublicKey;
use futures::future::BoxFuture;
use futures::FutureExt;
use launcher_api::bundle::LauncherBundle;
use launcher_extension_api::command::ExtensionCommand;
use launcher_macro::{command, register_commands};
use log::{error, info};
use rustyline::completion::{extract_word, Completer};
use rustyline::error::ReadlineError;
use rustyline::Config as LineConfig;
use rustyline::{CompletionType, Context, EditMode, Editor, OutputStreamType};
use rustyline_derive::{Helper, Highlighter, Hinter, Validator};
use std::collections::HashMap;
use std::fs;
use std::process::exit;
use std::sync::Arc;
use teloc::Resolver;
use tokio::sync::RwLock;

type CmdFn = for<'a> fn(Arc<LauncherServiceProvider>, &'a [&str]) -> BoxFuture<'a, ()>;

struct Command {
    name: &'static str,
    description: &'static str,
    func: CmdFn,
}

#[derive(Hinter, Helper, Validator, Highlighter)]
struct CommandHelper {
    service_provider: Arc<LauncherServiceProvider>,
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
    pub fn new(service_provider: Arc<LauncherServiceProvider>) -> Self {
        let extension_sp = service_provider.clone();
        let extension_service: &ExtensionService = extension_sp.resolve();
        CommandHelper {
            service_provider,
            commands: HashMap::new(),
            extension_commands: extension_service.get_commands(),
        }
    }

    pub async fn eval(&mut self, command: String) {
        let args: Vec<&str> = command.split(' ').collect();
        if args[0].eq("help") {
            self.show_help();
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
                    info!("Command not found. Use help.")
                }
            }
            Some(&c) => {
                let args = &args[1..];
                (c.func)(self.service_provider.clone(), args).await;
            }
        }
    }

    pub fn show_help(&self) {
        for command in self.commands.values() {
            info!("[launcher] {} - {}", command.name, command.description);
        }
        for (extension, commands) in &self.extension_commands {
            for (name, command) in commands {
                info!("[{}] {} - {}", extension, name, command.description);
            }
        }
    }

    pub fn new_command(&mut self, command: &'static Command) {
        self.commands.insert(&command.name, command);
    }
}

pub async fn run(server: Arc<LauncherServiceProvider>) -> Result<()> {
    tokio::spawn(async move {
        let rl_config = LineConfig::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .output_stream(OutputStreamType::Stdout)
            .build();
        let mut rl: Editor<CommandHelper> = Editor::with_config(rl_config);
        let mut helper = CommandHelper::new(server);
        register_commands!(rehash, sync, auth, bundle);
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
    Ok(())
}

#[command(description = "Update checksum of profile files")]
pub async fn rehash(sp: Arc<LauncherServiceProvider>, args: &[&str]) {
    hash::rehash(sp, args).await;
}

#[command(description = "Sync profile list between server and client")]
pub async fn sync(sp: Arc<LauncherServiceProvider>, _args: &[&str]) {
    let profile_service: Arc<RwLock<ProfileService>> = sp.resolve();
    let mut profile_service = profile_service.write().await;
    profile_service.profiles_data = profile::get_profiles_data();
    info!("Sync was successfully finished!");
}

#[command(description = "Authorize account with provided login and password")]
pub async fn auth(sp: Arc<LauncherServiceProvider>, args: &[&str]) {
    if args.len() < 2 {
        info!("Expected correct arguments number. Use auth <login> <password>")
    } else {
        let auth_provider: &AuthProvider = sp.resolve();
        match auth_provider.auth(args[0], args[1], "127.0.0.1").await {
            Ok(result) => {
                info!(
                    "Success auth: login '{}', uuid '{}', access_token '{}'",
                    args[0], result.uuid, result.access_token
                )
            }
            Err(error) => info!("Failed to auth: {}", error),
        }
    }
}

#[command(description = "Generate bundle file for launcher")]
pub async fn bundle(sp: Arc<LauncherServiceProvider>, _args: &[&str]) {
    info!("Start generate bundle for launcher...");
    let config: &Config = sp.resolve();
    let security_service: &SecurityService = sp.resolve();
    let public_key = PublicKey::from_secret(&security_service.secret_key);
    let bundle = LauncherBundle {
        game_dir: config.runtime.game_dir.clone(),
        websocket: config.websocket_url.clone(),
        ram: config.runtime.ram,
        project_name: config.project_name.clone(),
        public_key: public_key.to_bytes(),
        window: config.runtime.window.clone(),
    };
    let bundle_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("bundle.bin");
    match bundle_file {
        Ok(f) => {
            if let Err(e) = bincode::serialize_into(f, &bundle) {
                error!("Can't generate bundle file: {}", e);
            } else {
                info!("Bundle generated");
            }
        }
        Err(e) => {
            error!("Can't create bundle file: {}", e)
        }
    };
}
