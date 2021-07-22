// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::runtime::rpc::{format_callback, format_callback_result};
use crate::runtime::webview::{EventProxy, WebviewEvent};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub struct Invoke {
    pub message: InvokeMessage,
    pub resolver: InvokeResolver,
}

pub struct InvokeResolver {
    pub proxy: EventProxy,
    pub callback: String,
    pub error: String,
}

impl InvokeResolver {
    pub fn resolve<T: Serialize>(self, value: T) {
        Self::return_result(self.proxy, Ok(value).into(), self.callback, self.error)
    }

    pub fn resolve_result<T: Serialize>(self, value: anyhow::Result<T>) {
        match value {
            Ok(value) => self.resolve(value),
            Err(error) => self.reject(format!("{}", error)),
        }
    }

    pub fn reject<T: Serialize>(self, value: T) {
        Self::return_result(
            self.proxy,
            Result::<(), _>::Err(value.into()).into(),
            self.callback,
            self.error,
        )
    }

    pub fn return_result(
        proxy: EventProxy,
        response: InvokeResponse,
        success_callback: String,
        error_callback: String,
    ) {
        let callback_string = match format_callback_result(
            response.into_result(),
            success_callback,
            error_callback.clone(),
        ) {
            Ok(callback_string) => callback_string,
            Err(e) => format_callback(error_callback, &e.to_string())
                .expect("unable to serialize shortcut string to json"),
        };

        let _ = proxy.send_event(WebviewEvent::DispatchScript(callback_string));
    }
}

pub struct InvokeMessage {
    pub proxy: EventProxy,
    pub command: String,
    pub payload: Value,
}

#[derive(Deserialize)]
pub struct InvokePayload {
    pub callback: String,
    pub error: String,
    #[serde(flatten)]
    pub inner: Value,
}

#[derive(Debug)]
pub enum InvokeResponse {
    Ok(Value),
    Err(InvokeError),
}

impl InvokeResponse {
    #[inline(always)]
    pub fn into_result(self) -> Result<Value, Value> {
        match self {
            Self::Ok(v) => Ok(v),
            Self::Err(e) => Err(e.0),
        }
    }
}

impl<T: Serialize> From<Result<T, InvokeError>> for InvokeResponse {
    #[inline]
    fn from(result: Result<T, InvokeError>) -> Self {
        match result {
            Ok(ok) => match serde_json::to_value(ok) {
                Ok(value) => Self::Ok(value),
                Err(err) => Self::Err(InvokeError::from_serde_json(err)),
            },
            Err(err) => Self::Err(err),
        }
    }
}

impl From<InvokeError> for InvokeResponse {
    fn from(error: InvokeError) -> Self {
        Self::Err(error)
    }
}

#[derive(Debug)]
pub struct InvokeError(Value);

impl InvokeError {
    #[inline(always)]
    pub fn from_serde_json(error: serde_json::Error) -> Self {
        Self(Value::String(error.to_string()))
    }
}

impl<T: Serialize> From<T> for InvokeError {
    #[inline]
    fn from(value: T) -> Self {
        serde_json::to_value(value)
            .map(Self)
            .unwrap_or_else(Self::from_serde_json)
    }
}
