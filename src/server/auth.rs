use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::LaunchServer;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::http::StatusCode;
use warp::Reply;

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
// TODO REMOVE Box to warp::reply::with_status()

pub(crate) async fn join(
    request: JoinRequest,
    data: Arc<RwLock<LaunchServer>>,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let data = data.read().await;
    let provide = &data.config.auth;
    let entry = provide.get_entry(&request.selected_profile).await;
    match entry {
        Ok(e) => {
            if e.access_token.is_some() && e.access_token.unwrap().eq(&request.access_token) {
                provide
                    .update_server_id(&request.selected_profile, &request.server_id)
                    .await;
                Ok(Box::new(StatusCode::OK))
            } else {
                Ok(Box::new(warp::reply::json(&serde_json::json!({
                     "error": "accessToken error",
                     "errorMessage": "Access token not equals"
                }))))
            }
        }
        Err(error) => Ok(Box::new(warp::reply::json(&serde_json::json!({
             "error": error.msg
             ,
             "errorMessage": "Подробное описание, ОТОБРАЖАЕМОЕ В КЛИЕНТЕ!",
             "cause": "Причина ошибки (опционально)"
        })))),
    }
}

pub(crate) async fn has_join(
    request: HasJoinRequest,
    data: Arc<RwLock<LaunchServer>>,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let data = data.read().await;
    let texture = &data.config.texture;
    let auth = &data.config.auth;
    let entry = auth.get_entry_from_name(&request.username).await;
    match entry {
        Err(_e) => Ok(Box::new(StatusCode::BAD_REQUEST)),
        Ok(e) => {
            if e.server_id.is_some() && e.server_id.clone().unwrap().eq(&request.server_id) {
                let texture =
                    base64::encode(&texture.get_textures_property(&e).to_string()).to_string();
                Ok(Box::new(warp::reply::json(&serde_json::json!({
                    "id":  e.uuid.to_simple().encode_lower(&mut Uuid::encode_buffer()),
                    "name": request.username,
                    "properties": [
                        {
                            "name": "textures",
                            "value": texture
                        }
                    ]
                }))))
            } else {
                Ok(Box::new(StatusCode::BAD_REQUEST))
            }
        }
    }
}
