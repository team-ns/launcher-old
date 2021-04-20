use crate::command::CommandRegister;

use crate::connection::Client;
use crate::launcher::message::ClientMessage;
pub use anyhow::{anyhow, Context, Error, Result};

pub mod command;
pub mod connection;
pub mod launcher;

pub trait LauncherExtension: Send + Sync {
    fn init(&self) -> Result<()> {
        Ok(())
    }
    fn register_command(&self, _register: &mut CommandRegister) {}
    fn handle_message(&self, _message: &ClientMessage, _client: &mut Client) -> Result<()> {
        Ok(())
    }
}
