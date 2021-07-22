use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::runtime::arg::InvokeResolver;
use crate::runtime::webview::{EventProxy, WebviewEvent};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
    Hide,
    Show,
    Exit,
}

impl Cmd {
    pub fn run(self, resolver: InvokeResolver, proxy: EventProxy) {
        match self {
            Cmd::Hide => resolver.resolve_result(
                proxy
                    .send_event(WebviewEvent::HideWindow)
                    .context("Can't hide window"),
            ),
            Cmd::Show => resolver.resolve_result(
                proxy
                    .send_event(WebviewEvent::ShowWindow)
                    .context("Can't show window"),
            ),
            Cmd::Exit => {
                resolver.resolve_result(proxy.send_event(WebviewEvent::Exit).context("Can't exit"))
            }
        }
    }
}
