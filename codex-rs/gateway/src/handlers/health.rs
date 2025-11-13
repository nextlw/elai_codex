//! Health check handler

use crate::error::GatewayResult;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::Value;
use serde_json::json;

/// Health check endpoint
///
/// Returns a simple JSON response indicating the service is healthy
///
/// ## Response
///
/// ```json
/// {
///   "status": "healthy"
/// }
/// ```
pub async fn health_check(
    State(_state): State<AppState>,
) -> GatewayResult<(StatusCode, Json<Value>)> {
    tracing::debug!("Health check requested");

    let response = json!({
        "status": "healthy"
    });

    Ok((StatusCode::OK, Json(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GatewayConfig;

    #[tokio::test]
    async fn test_health_check() -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new(GatewayConfig::default()).await?;
        let result = health_check(State(state)).await;

        assert!(result.is_ok());
        let (status, json_response) = result.unwrap();
        assert_eq!(status, StatusCode::OK);

        let value = json_response.0;
        assert_eq!(value["status"], "healthy");
        Ok(())
    }
}
