// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use anyhow::Result;
use serde::Serialize;
use serde_json::value::RawValue;

const MIN_JSON_PARSE_LEN: usize = 10_240;
const MAX_JSON_STR_LEN: usize = usize::pow(2, 30) - 2;

fn escape_json_parse(json: &RawValue) -> String {
    let json = json.get();

    let mut s = String::with_capacity(json.len() + 14);
    s.push_str("JSON.parse('");

    let mut last = 0;
    for (idx, _) in json.match_indices(|c| c == '\\' || c == '\'') {
        s.push_str(&json[last..idx]);
        s.push('\\');
        last = idx;
    }

    s.push_str(&json[last..]);
    s.push_str("')");
    s
}

pub fn format_callback<T: Serialize, S: AsRef<str>>(function_name: S, arg: &T) -> Result<String> {
    macro_rules! format_callback {
    ( $arg:expr ) => {
      format!(
        r#"
          if (window["{fn}"]) {{
            window["{fn}"]({arg})
          }} else {{
            console.warn("[TAURI] Couldn't find callback id {fn} in window. This happens when the app is reloaded while Rust is running an asynchronous operation.")
          }}
        "#,
        fn = function_name.as_ref(),
        arg = $arg
      )
    }
  }

    let string = serde_json::to_string(arg)?;
    let raw = RawValue::from_string(string)?;

    let json = raw.get();
    let first = json.as_bytes()[0];

    #[cfg(debug_assertions)]
    if first == b'"' {
        debug_assert!(
            json.len() < MAX_JSON_STR_LEN,
            "passing a callback string larger than the max JavaScript literal string size"
        )
    }

    Ok(
        if json.len() > MIN_JSON_PARSE_LEN && (first == b'{' || first == b'[') {
            let escaped = escape_json_parse(&raw);
            if escaped.len() < MAX_JSON_STR_LEN {
                format_callback!(escaped)
            } else {
                format_callback!(json)
            }
        } else {
            format_callback!(json)
        },
    )
}

pub fn format_callback_result<T: Serialize, E: Serialize>(
    result: Result<T, E>,
    success_callback: impl AsRef<str>,
    error_callback: impl AsRef<str>,
) -> Result<String> {
    match result {
        Ok(res) => format_callback(success_callback, &res),
        Err(err) => format_callback(error_callback, &err),
    }
}
