// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::runtime::arg::InvokeResolver;
use crate::runtime::webview::{EventProxy, WebviewEvent};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
    Listen {
        event: String,
        handler: String,
    },
    #[serde(rename_all = "camelCase")]
    Unlisten {
        event_id: u64,
    },
}

impl Cmd {
    pub fn run(self, resolver: InvokeResolver, proxy: EventProxy) {
        match self {
            Self::Listen { event, handler } => {
                resolver.resolve_result(listen(event, handler, proxy));
            }
            Cmd::Unlisten { event_id } => {
                resolver.resolve_result(
                    proxy
                        .send_event(WebviewEvent::DispatchScript(unlisten_js(event_id)))
                        .context("Can't send event"),
                );
            }
        }
    }
}

pub fn listen(event: String, handler: String, proxy: EventProxy) -> Result<u64> {
    let event_id = rand::random();
    proxy.send_event(WebviewEvent::DispatchScript(listen_js(
        event, event_id, handler,
    )))?;
    Ok(event_id)
}

pub fn unlisten_js(event_id: u64) -> String {
    format!(
        "
      for (var event in (window['{listeners}'] || {{}})) {{
        var listeners = (window['{listeners}'] || {{}})[event]
        if (listeners) {{
          window['{listeners}'][event] = window['{listeners}'][event].filter(function (e) {{ e.id !== {event_id} }})
        }}
      }}
    ",
        listeners = "08b8adb2-276a-49e0-b5d1-67ee5c381946",
        event_id = event_id,
    )
}

pub fn listen_js(event: String, event_id: u64, handler: String) -> String {
    format!(
        "if (window['{listeners}'] === void 0) {{
      window['{listeners}'] = Object.create(null)
    }}
    if (window['{listeners}']['{event}'] === void 0) {{
      window['{listeners}']['{event}'] = []
    }}
    window['{listeners}']['{event}'].push({{
      id: {event_id},
      handler: window['{handler}']
    }});
    for (let i = 0; i < (window['{queue}'] || []).length; i++) {{
      const e = window['{queue}'][i];
      window['{emit}'](e.eventData, true)
    }}
  ",
        queue = "53a38c36-7040-4b88-aca2-b7390bbd3fd6",
        listeners = "08b8adb2-276a-49e0-b5d1-67ee5c381946",
        emit = "b03ebfab-8145-44ac-a4b6-d800ddfa3bba",
        event = event,
        event_id = event_id,
        handler = handler
    )
}
