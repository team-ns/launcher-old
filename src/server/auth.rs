use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRequest {
    access_token: String,
    server_id: String,
    selected_profile: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HasJoinRequest {
    username: String,
    server_id: String,
}


pub(crate) async fn join(data: web::Data<Config>, request: web::Json<JoinRequest>) -> HttpResponse {
    let provide = data.auth.get_provide();
    let entry = provide.get_entry(&request.selected_profile).await;
    match entry {
        Ok(e) => {
            if e.access_token.is_some() && e.access_token.unwrap().eq(&request.access_token) {
                provide.update_server_id(&request.selected_profile, &request.server_id).await;
                HttpResponse::Ok().finish()
            } else {
                HttpResponse::Ok().json(serde_json::json!({
                 "error": "accessToken error",
                 "errorMessage": "Access token not equals"
            }))
            }
        }
        Err(error) => {
            HttpResponse::Ok().json(serde_json::json!({
                 "error": error.message,
                 "errorMessage": "Подробное описание, ОТОБРАЖАЕМОЕ В КЛИЕНТЕ!",
                 "cause": "Причина ошибки (опционально)"
            }))
        }
    }
}

pub(crate) async fn has_join(data: web::Data<Config>, form: web::Query<HasJoinRequest>) -> HttpResponse {
    let texture = &data.texture;
    let auth = &data.auth.get_provide();
    let entry = auth.get_entry_from_name(&form.username).await;
    match entry {
        Err(_e) => HttpResponse::Ok().finish(),
        Ok(e) => {
            if e.server_id.is_some() && e.server_id.clone().unwrap().eq(&form.server_id) {
                let texture = base64::encode(&texture.get_textures_property(&e).to_string()).to_string();
                HttpResponse::Ok().json(serde_json::json!({
                    "id":  e.uuid.to_simple().encode_lower(&mut Uuid::encode_buffer()),
                    "name": form.username,
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