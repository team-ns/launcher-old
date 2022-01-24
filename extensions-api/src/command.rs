use std::collections::HashMap;

pub struct ExtensionCommand {
    pub name: String,
    pub description: String,
    pub executor: Box<dyn ExtensionCommandExecutor>,
}

pub trait ExtensionCommandExecutor: Send {
    fn execute(&self, args: &[&str]);
}

#[derive(Default)]
pub struct CommandRegister {
    extension_commands: HashMap<String, ExtensionCommand>,
}

impl CommandRegister {
    pub fn register(
        &mut self,
        name: &str,
        description: &str,
        command: Box<dyn ExtensionCommandExecutor>,
    ) {
        let command = ExtensionCommand {
            name: name.to_string(),
            description: description.to_string(),
            executor: command,
        };
        self.extension_commands
            .insert(command.name.clone(), command);
    }

    pub fn into_commands(self) -> HashMap<String, ExtensionCommand> {
        self.extension_commands
    }
}
