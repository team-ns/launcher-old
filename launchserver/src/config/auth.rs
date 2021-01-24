use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonAuthConfig {
    pub auth_url: String,
    pub entry_url: String,
    pub update_server_id_url: String,
    pub update_access_token_url: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SqlAuthConfig {
    pub connection_url: String,
    pub fetch_entry_username_query: String,
    pub fetch_entry_uuid_query: String,
    pub auth_query: String,
    pub auth_message: String,
    pub update_server_id_query: String,
    pub update_access_token_query: String,
}
