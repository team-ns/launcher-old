use crate::command::CommandRegister;

use crate::connection::Client;
use crate::launcher::message::{ClientMessage, ServerMessage};
pub use anyhow::{anyhow, Context, Error, Result};

pub mod command;
pub mod connection;
pub mod launcher;

pub trait LauncherExtension: Send + Sync {
    fn init(&self) -> Result<()> {
        Ok(())
    }
    fn register_command(&self, _register: &mut CommandRegister) {}
    fn handle_connection(&self, _client: &Client) {}
    fn pre_handle_message(
        &self,
        _message: &ClientMessage,
        _client: &mut Client,
    ) -> Result<Option<ServerMessage>> {
        Ok(None)
    }
    fn post_handle_message(
        &self,
        _request_message: &ClientMessage,
        _client: &mut Client,
        _response_message: &ServerMessage,
    ) -> Result<Option<ServerMessage>> {
        Ok(None)
    }
}
