use std::collections::HashMap;
use std::thread;

use log::info;
use rustyline::{CompletionType, Context, EditMode, Editor, OutputStreamType};
use rustyline::completion::{Completer, extract_word, Pair};
use rustyline::Config as LineConfig;
use rustyline::error::ReadlineError;
use rustyline::line_buffer::LineBuffer;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline_derive::{Completer, Helper, Highlighter, Hinter, Validator};

use crate::config::Config;

type CmdFn = Box<dyn Fn(&Config, &[&str])>;

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

#[derive(Hinter ,Helper, Validator, Highlighter)]
struct CommandHelper {
    config: Config,
    commands: HashMap<String, Command>,
}



impl Completer for CommandHelper {
    type Candidate = String;


    fn complete(&self, line: &str, pos: usize, ctx: &Context) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let (word_start, word_to_complete) = extract_word(line, pos, None, &[32u8][..]);
        let results = self.commands.keys()
            .filter_map(|cmd| {
                if cmd.starts_with(line){
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
    pub fn new(config: Config) -> Self {
        CommandHelper {
            config,
            commands: HashMap::new(),
        }
    }

    pub fn eval(&mut self, command: String) {
        let args: Vec<&str> = command.split(' ').collect();
        let selected_command = self.commands.get(&args[0].to_string());
        match selected_command {
            None => {
                println!("Command not found. Use help.")
            }
            Some(c) => {
                (c.func)(&self.config, &args[1..])
            }
        }
    }

    pub fn new_command<F>(&mut self, name: &str, description: &str, command: F)
    where F: Fn(&Config, &[&str]) + 'static {
        self.commands.insert(name.to_string(), Command::new(name, description,  Box::new(command)));
    }
}

pub fn start(config: Config) {
    thread::spawn(move || {
        let rl_config = LineConfig::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .output_stream(OutputStreamType::Stdout)
            .build();
        let mut rl: Editor<CommandHelper> = Editor::with_config(rl_config);
        let mut helper = CommandHelper::new(config.clone());
        register_command(&mut helper);
        rl.set_helper(Some(helper));
        loop {
            let readline = rl.readline("");
            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str());
                    rl.helper_mut().unwrap().eval(line.trim_end_matches("\n").to_string());

                }
                Err(ReadlineError::Interrupted) => {
                    println!("End");
                    break;
                }
                _ => {}
            }
        }
   });
}

fn register_command(helper: &mut CommandHelper) {
    helper.new_command("some", "some command", some_command);
}

fn some_command(config: &Config, args: &[&str]) {
    println!("Test:  {:#?}", args)
}
