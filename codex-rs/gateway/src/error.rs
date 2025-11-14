//! Error types for the Codex Gateway

use thiserror::Error;

/// Errors that can occur in the gateway
#[derive(Error, Debug)]
pub enum GatewayError {
    /// HTTP-related errors
    #[error("HTTP error: {0}")]
    Http(#[from] axum::http::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// WebSocket errors
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Server start errors
    #[error("Server start error: {0}")]
    ServerStart(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Internal server errors
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Service unavailable errors
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Authentication/Authorization errors
    #[error("Auth error: {0}")]
    Auth(String),

    /// Timeout errors
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Generic errors from anyhow
    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),

    /// Payload too large error
    #[error("Request body too large for endpoint '{path}' (max {max_size} bytes allowed)")]
    PayloadTooLarge {
        /// Maximum allowed size in bytes
        max_size: usize,
        /// Actual size attempted (if known)
        actual_size: Option<usize>,
        /// Endpoint path where the violation occurred
        path: String,
    },

    /// Invalid request error (malformed or invalid parameters)
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Result type alias for gateway operations
pub type GatewayResult<T> = Result<T, GatewayError>;

impl axum::response::IntoResponse for GatewayError {
    fn into_response(self) -> axum::response::Response {
        use axum::Json;
        use axum::http::StatusCode;

        let (status, error_message) = match self {
            GatewayError::Http(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            GatewayError::Json(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            GatewayError::WebSocket(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            GatewayError::ServerStart(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            GatewayError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            GatewayError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            GatewayError::ServiceUnavailable(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
            }
            GatewayError::Auth(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            GatewayError::Timeout(_) => (StatusCode::REQUEST_TIMEOUT, self.to_string()),
            GatewayError::Generic(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            GatewayError::PayloadTooLarge { .. } => {
                (StatusCode::PAYLOAD_TOO_LARGE, self.to_string())
            }
            GatewayError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };

        let body = Json(serde_json::json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}
