use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthProvider;
use crate::config::Config;
use crate::LauncherServiceProvider;
use ntex::web;
use ntex::web::types::Json;
use ntex::web::{HttpRequest, HttpResponse};
use teloc::Resolver;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HasJoinRequest {
    username: String,
    server_id: String,
}

pub async fn has_join(
    body: Json<HasJoinRequest>,
    _req: HttpRequest,
    sp: web::types::Data<LauncherServiceProvider>,
) -> HttpResponse {
    let config: &Config = sp.resolve();
    let texture = &config.texture;
    let auth: &AuthProvider = sp.resolve();
    let entry = auth.get_entry_from_name(&body.username).await;
    match entry {
        Err(_e) => HttpResponse::BadRequest().finish(),
        Ok(e) => {
            if e.server_id.is_some() && e.server_id.clone().unwrap().eq(&body.server_id) {
                let texture = base64::encode(&texture.get_textures_property(&e).to_string());
                HttpResponse::Ok().json(&serde_json::json!({
                    "id":  e.uuid.to_simple().encode_lower(&mut Uuid::encode_buffer()),
                    "name": body.username,
                    "properties": [
                        {
                            "name": "textures",
                            "value": texture
                        }
                    ]
                }))
            } else {
                HttpResponse::BadRequest().finish()
            }
        }
    }
}
