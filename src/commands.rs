use launcher_api::profile::Profile;
use launcher_api::validation::{HashedFile, HashedProfile};
use rustyline::completion::{extract_word, Completer};
use rustyline::error::ReadlineError;
use rustyline::Config as LineConfig;
use rustyline::{CompletionType, Context, EditMode, Editor, OutputStreamType};
use rustyline_derive::{Helper, Highlighter, Hinter, Validator};
use std::collections::HashMap;
use std::fs::File;
use std::ops::DerefMut;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::RwLock;
use walkdir::{DirEntry, WalkDir};

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
                    Some(String::from(cmd))
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
}

pub fn rehash(server: &mut LaunchServer, args: &[&str]) {
    //duplicate files map
    let mut hashed_libs = HashMap::new();
    let mut hashed_natives = HashMap::new();

    //hashed profiles
    let mut hashed_profiles: HashMap<String, HashedProfile> = HashMap::new();

    //profile list
    let profiles: Vec<Profile> = WalkDir::new("static/profiles")
        .min_depth(2)
        .max_depth(3)
        .into_iter()
        .flat_map(|v| v.ok())
        .filter(|e| {
            e.metadata().map(|m| m.is_file()).unwrap_or(false) && e.file_name().eq("profile.json")
        })
        .flat_map(|e| File::open(e.into_path()).ok())
        .flat_map(|f| serde_json::from_reader(f).ok())
        .collect();

    fn fill_map(iter: impl Iterator<Item = DirEntry>, map: &mut HashMap<String, HashedFile>) {
        for file in iter {
            let path = file.path();
            let strip_path = String::from(path.strip_prefix("static/").unwrap().to_str().unwrap());
            map.insert(strip_path, HashedFile::new(path.to_string_lossy().as_ref()));
        }
    }

    //fill duplicate map
    let lib_iter = WalkDir::new("static/libs")
        .min_depth(1)
        .into_iter()
        .flat_map(|e| e.ok())
        .filter(|e| e.metadata().map(|m| m.is_file()).unwrap_or(false))
        .into_iter();
    fill_map(lib_iter, &mut hashed_libs);

    //fill native list
    let native_versions = WalkDir::new("static/natives")
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.metadata().map(|m| m.is_dir()).unwrap_or(false))
        .into_iter();

    for version in native_versions {
        let mut hashed_native = HashMap::new();

        let native_iter = WalkDir::new(version.path())
            .min_depth(1)
            .into_iter()
            .flat_map(|e| e.ok())
            .filter(|e| e.metadata().map(|m| m.is_file()).unwrap_or(false))
            .into_iter();
        fill_map(native_iter, &mut hashed_native);

        hashed_natives.insert(
            String::from(
                version
                    .path()
                    .strip_prefix("static/natives/")
                    .unwrap()
                    .to_str()
                    .unwrap(),
            ),
            hashed_native,
        );
    }

    for profile in &profiles {
        //create profiles and hash non duplicate files
        let mut hashed_profile = HashedProfile::new();

        let file_iter = WalkDir::new(format!("static/profiles/{}", profile.name))
            .min_depth(1)
            .into_iter()
            .flat_map(|e| e.ok())
            .filter(|e| e.metadata().map(|m| m.is_file()).unwrap_or(false))
            .into_iter();
        fill_map(file_iter, &mut hashed_profile);

        //fill libs from duplicate map
        for lib in &profile.libraries {
            //TODO fix possible error with sync
            let lib = format!("libs/{}", lib);
            hashed_profile.insert(lib.clone(), hashed_libs.get(&lib).unwrap().clone());
        }
        //fill natives from natives map
        for native in hashed_natives.get(&profile.version).unwrap() {
            hashed_profile.insert(String::from(native.0), native.1.clone());
        }
        hashed_profiles.insert(String::from(&profile.name), hashed_profile);
    }

    server.security.profiles = hashed_profiles;
}
