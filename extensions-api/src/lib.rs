use crate::command::CommandRegister;

pub use anyhow::{anyhow, Context, Error, Result};

pub mod command;

pub trait LauncherExtension: Send + Sync {
    fn register_command(&self, _register: &mut CommandRegister) {}
    fn init(&self) -> Result<()> {
        Ok(())
    }
    fn pre_auth(&self, _login: &str, _password: &str, _ip: &str) -> Result<()> {
        Ok(())
    }
}
