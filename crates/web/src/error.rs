use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// What an API handler can fail with. Answers as `{"error": "..."}`.
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized,
    /// Also used for things in servers the admin doesn't manage, so their existence isn't revealed.
    NotFound,
    Internal(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "faça login com o Discord".to_string()),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "não encontrado".to_string()),
            ApiError::Internal(err) => {
                tracing::error!("web request failed: {err:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "erro interno".to_string())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(err: E) -> Self {
        ApiError::Internal(err.into())
    }
}
