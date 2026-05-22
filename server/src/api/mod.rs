use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use protocol::models::{CommandRequest, CommandType};
use std::sync::Arc;
use uuid::Uuid;
use serde::Deserialize;
use crate::AppState;

#[derive(Deserialize)]
pub struct CommandInput {
    pub command: CommandType,
    pub payload: serde_json::Value,
}

pub async fn send_command(
    Path(agent_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(input): Json<CommandInput>,
) -> impl IntoResponse {
    let request_id = Uuid::new_v4();
    let req = CommandRequest {
        request_id,
        command: input.command,
        payload: input.payload,
    };

    match state.session_manager.send_command(&agent_id, req).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => {
            log::error!("Failed to send command to {}: {:?}", agent_id, e);
            match e {
                crate::websocket::SessionError::NotFound(_) => {
                    (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Agent not found" }))).into_response()
                }
                crate::websocket::SessionError::Timeout => {
                    (StatusCode::GATEWAY_TIMEOUT, Json(serde_json::json!({ "error": "Command timed out" }))).into_response()
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("{:?}", e) }))).into_response(),
            }
        }
    }
}
