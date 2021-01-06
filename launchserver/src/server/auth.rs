use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::LaunchServer;
use serde_json::Value;
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

pub(crate) async fn has_join(
    request: HasJoinRequest,
    data: Arc<RwLock<LaunchServer>>,
) -> Result<impl Reply, warp::Rejection> {
    let data = data.read().await;
    let texture = &data.config.texture;
    let auth = &data.config.auth;
    let entry = auth.get_entry_from_name(&request.username).await;
    match entry {
        Err(_e) => Ok(warp::reply::with_status(
            warp::reply::json(&Value::default()),
            StatusCode::BAD_REQUEST,
        )),
        Ok(e) => {
            if e.server_id.is_some() && e.server_id.clone().unwrap().eq(&request.server_id) {
                let texture = base64::encode(&texture.get_textures_property(&e).to_string());
                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "id":  e.uuid.to_simple().encode_lower(&mut Uuid::encode_buffer()),
                        "name": request.username,
                        "properties": [
                            {
                                "name": "textures",
                                "value": texture
                            }
                        ]
                    })),
                    StatusCode::OK,
                ))
            } else {
                Ok(warp::reply::with_status(
                    warp::reply::json(&Value::default()),
                    StatusCode::BAD_REQUEST,
                ))
            }
        }
    }
}
