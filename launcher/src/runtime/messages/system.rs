use crate::runtime::arg::InvokeResolver;
use anyhow::Result;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
    GetTotalRam,
}

impl Cmd {
    pub fn run(self, resolver: InvokeResolver) {
        match self {
            Cmd::GetTotalRam => resolver.resolve_result(get_ram()),
        };
    }
}

fn get_ram() -> Result<u64> {
    let mem_info = sys_info::mem_info()?;
    Ok(mem_info.total)
}
