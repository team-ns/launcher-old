use crate::command::CommandRegister;

pub use anyhow::{anyhow, Context, Error, Result};

pub use uuid::Uuid;

pub mod channel;
pub mod command;
pub mod launcher;

pub trait LauncherExtension: Send + Sync {
    fn register_command(&self, _register: &mut CommandRegister) {}
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
