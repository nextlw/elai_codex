//! Webhook handler

use crate::error::GatewayResult;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::Value;
use serde_json::json;

/// Webhook endpoint
///
/// Accepts generic webhook payloads and processes them asynchronously.
/// This is a placeholder implementation that logs the payload and returns an acknowledgment.
///
/// Future iterations will implement:
/// - Webhook signature verification
/// - Event type routing
/// - Asynchronous processing queues
/// - Retry mechanisms
/// - Dead letter queues
///
/// ## Request
///
/// Accepts any JSON payload.
///
/// ## Response
///
/// Returns HTTP 202 Accepted with acknowledgment.
pub async fn handle_webhook(
    State(_state): State<AppState>,
    Json(payload): Json<Value>,
) -> GatewayResult<(StatusCode, Json<Value>)> {
    tracing::info!("Webhook received");
    tracing::debug!("Webhook payload: {}", payload);

    // Log webhook details for debugging
    if let Some(event_type) = payload.get("type").and_then(|v| v.as_str()) {
        tracing::info!("Webhook event type: {}", event_type);
    }

    if let Some(source) = payload.get("source").and_then(|v| v.as_str()) {
        tracing::info!("Webhook source: {}", source);
    }

    // Placeholder: In future iterations, this will:
    // 1. Validate webhook signature
    // 2. Route to appropriate service based on event type
    // 3. Queue for async processing
    // 4. Return appropriate response based on processing requirements

    let response = json!({
        "status": "accepted",
        "message": "Webhook received and queued for processing",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Ok((StatusCode::ACCEPTED, Json(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GatewayConfig;

    #[tokio::test]
    async fn test_webhook_handler() -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new(GatewayConfig::default()).await?;

        let webhook_payload = json!({
            "type": "test_event",
            "source": "test_service",
            "data": {
                "message": "test webhook"
            }
        });

        let result = handle_webhook(State(state), Json(webhook_payload)).await;

        assert!(result.is_ok());
        let (status, json_response) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);

        let value = json_response.0;
        assert_eq!(value["status"], "accepted");
        assert!(value["message"].is_string());
        assert!(value["timestamp"].is_string());
        Ok(())
    }

    #[tokio::test]
    async fn test_webhook_with_minimal_payload() -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new(GatewayConfig::default()).await?;

        let minimal_payload = json!({
            "data": "minimal"
        });

        let result = handle_webhook(State(state), Json(minimal_payload)).await;

        assert!(result.is_ok());
        let (status, _) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        Ok(())
    }

    #[tokio::test]
    async fn test_webhook_with_empty_payload() -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new(GatewayConfig::default()).await?;

        let empty_payload = json!({});

        let result = handle_webhook(State(state), Json(empty_payload)).await;

        assert!(result.is_ok());
        let (status, _) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        Ok(())
    }
}
