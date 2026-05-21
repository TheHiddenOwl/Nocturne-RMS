use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    http::StatusCode,
};
use protocol::API_KEY_HEADER;

pub async fn auth_middleware(
    request: Request,
    next: Next,
    api_keys: Vec<String>,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(API_KEY_HEADER)
        .and_then(|v| v.to_str().ok());

    if let Some(key) = auth_header {
        if api_keys.contains(&key.to_string()) {
            return Ok(next.run(request).await);
        }
    }

    log::warn!("Unauthorized connection attempt");
    Err(StatusCode::UNAUTHORIZED)
}
