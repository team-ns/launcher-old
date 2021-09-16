use ntex::web;
use ntex::web::types::Json;
use ntex::web::{HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use teloc::Resolver;
use uuid::Uuid;

use crate::auth::{AuthProvider, Entry};
use crate::config::Config;
use crate::LauncherServiceProvider;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HasJoinRequest {
    username: String,
    server_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsernameToUuidResponse {
    name: String,
    id: String,
}

pub async fn has_join(
    body: Json<HasJoinRequest>,
    _req: HttpRequest,
    sp: web::types::Data<LauncherServiceProvider>,
) -> HttpResponse {
    let config: &Config = sp.resolve();
    let auth: &AuthProvider = sp.resolve();
    let entry = auth.get_entry_from_name(&body.username).await;
    match entry {
        Err(_e) => HttpResponse::BadRequest().finish(),
        Ok(e) => {
            if e.server_id.is_some() && e.server_id.clone().unwrap().eq(&body.server_id) {
                HttpResponse::Ok().json(&get_player_profile(&e, config))
            } else {
                HttpResponse::BadRequest().finish()
            }
        }
    }
}

pub async fn uuid_to_profile(
    path: web::types::Path<String>,
    _req: HttpRequest,
    sp: web::types::Data<LauncherServiceProvider>,
) -> HttpResponse {
    let uuid = match Uuid::parse_str(path.as_ref()) {
        Ok(uuid) => uuid,
        Err(e) => {
            log::error!("Can't parse player uuid {}: {}", path.as_ref(), e);
            return HttpResponse::BadRequest().finish();
        }
    };
    let config: &Config = sp.resolve();
    let auth: &AuthProvider = sp.resolve();
    let entry = auth.get_entry(&uuid).await;
    match entry {
        Err(_e) => HttpResponse::BadRequest().finish(),
        Ok(e) => HttpResponse::Ok().json(&get_player_profile(&e, config)),
    }
}

fn get_player_profile(e: &Entry, config: &Config) -> serde_json::Value {
    let texture = &config.texture;
    let texture = base64::encode(&texture.get_textures_property(e).to_string());
    serde_json::json!({
                    "id":  e.uuid.to_simple().encode_lower(&mut Uuid::encode_buffer()),
                    "name": e.username,
                    "properties": [
                        {
                            "name": "textures",
                            "value": texture
                        }
                    ]
    })
}

pub async fn username_to_uuid(
    body: Json<Vec<String>>,
    _req: HttpRequest,
    sp: web::types::Data<LauncherServiceProvider>,
) -> HttpResponse {
    let auth: &AuthProvider = sp.resolve();
    let response_body = {
        let mut uuids = Vec::with_capacity(body.0.len());
        for username in body.0.iter() {
            match auth.get_entry_from_name(username).await {
                Ok(entry) => {
                    let username_to_uuid = UsernameToUuidResponse {
                        name: entry.username,
                        id: entry
                            .uuid
                            .to_simple()
                            .encode_lower(&mut Uuid::encode_buffer())
                            .to_string(),
                    };
                    uuids.push(username_to_uuid);
                }
                Err(e) => {
                    log::error!("Can't get player uuid with username {}: {}", username, e);
                    return HttpResponse::BadRequest().finish();
                }
            }
        }
        uuids
    };
    HttpResponse::Ok().json(&response_body)
}
