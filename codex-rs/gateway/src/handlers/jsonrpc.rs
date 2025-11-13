//! JSON-RPC handler

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;
use tracing::debug;
use tracing::error;
use tracing::info;

use crate::error::GatewayResult;
use crate::services::CodexService;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data,
            }),
            id,
        }
    }

    pub fn internal_error(id: Option<Value>, message: String) -> Self {
        Self::error(id, -32603, message, None)
    }

    pub fn invalid_params(id: Option<Value>, message: String) -> Self {
        Self::error(id, -32602, message, None)
    }
}

/// JSON-RPC endpoint
///
/// Accepts JSON-RPC requests and processes them through the Codex system.
/// Supports methods: execute, chat, status, cancel, list_models, auth_status, ping, echo.
///
/// ## Request
///
/// Expects a JSON-RPC 2.0 formatted request body.
///
/// ## Response
///
/// Returns a JSON-RPC 2.0 formatted response.
pub async fn handle_jsonrpc(
    State(state): State<AppState>,
    Json(request): Json<JsonRpcRequest>,
) -> GatewayResult<(StatusCode, Json<JsonRpcResponse>)> {
    info!(
        "Received JSON-RPC request: method={}, id={:?}",
        request.method, request.id
    );
    debug!("Request params: {:?}", request.params);

    // Validate JSON-RPC version
    if request.jsonrpc != "2.0" {
        let response = JsonRpcResponse::error(
            request.id,
            -32600,
            "Invalid Request: JSON-RPC version must be '2.0'".to_string(),
            None,
        );
        return Ok((StatusCode::OK, Json(response)));
    }

    // Get CodexService from state
    let codex_service = &state.codex_service;

    let response = match request.method.as_str() {
        "conversation.prompt" => {
            info!("Processing conversation.prompt request");
            process_execute(codex_service, &request).await
        }
        "conversation.status" => {
            info!("Processing conversation.status request");
            process_status(codex_service, &request).await
        }
        "conversation.cancel" => {
            info!("Processing conversation.cancel request");
            process_cancel(codex_service, &request).await
        }
        _ => {
            error!("Unknown method: {}", request.method);
            JsonRpcResponse::error(
                request.id,
                -32601,
                format!("Method '{}' not found", request.method),
                Some(json!({
                    "available_methods": [
                        "conversation.prompt",
                        "conversation.status",
                        "conversation.cancel"
                    ]
                })),
            )
        }
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Process execute request - main AI prompt processing
async fn process_execute(service: &CodexService, request: &JsonRpcRequest) -> JsonRpcResponse {
    let params = match &request.params {
        Some(p) => p,
        None => {
            return JsonRpcResponse::invalid_params(
                request.id.clone(),
                "Missing required 'params' field".to_string(),
            );
        }
    };

    let prompt = match params.get("prompt") {
        Some(Value::String(p)) => p,
        Some(_) => {
            return JsonRpcResponse::invalid_params(
                request.id.clone(),
                "Parameter 'prompt' must be a string".to_string(),
            );
        }
        None => {
            return JsonRpcResponse::invalid_params(
                request.id.clone(),
                "Missing required parameter 'prompt'".to_string(),
            );
        }
    };

    let session_id = params.get("session_id").and_then(|v| v.as_str());

    match service.execute_prompt(prompt, session_id).await {
        Ok(result) => JsonRpcResponse::success(request.id.clone(), result),
        Err(e) => {
            error!("Execute failed: {}", e);
            JsonRpcResponse::internal_error(request.id.clone(), format!("Execute failed: {e}"))
        }
    }
}

/// Process status request - get processing status
async fn process_status(service: &CodexService, request: &JsonRpcRequest) -> JsonRpcResponse {
    let params = match &request.params {
        Some(p) => p,
        None => {
            return JsonRpcResponse::invalid_params(
                request.id.clone(),
                "Missing required 'params' field".to_string(),
            );
        }
    };

    let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return JsonRpcResponse::invalid_params(
                request.id.clone(),
                "Missing required parameter 'session_id'".to_string(),
            );
        }
    };

    match service.get_session_status(session_id).await {
        Ok(Some(status)) => JsonRpcResponse::success(request.id.clone(), json!(status)),
        Ok(None) => JsonRpcResponse::success(
            request.id.clone(),
            json!({ "status": "not_found", "session_id": session_id }),
        ),
        Err(e) => {
            error!("Status check failed: {}", e);
            JsonRpcResponse::internal_error(request.id.clone(), format!("Status check failed: {e}"))
        }
    }
}

/// Process cancel request - cancel ongoing processing
async fn process_cancel(service: &CodexService, request: &JsonRpcRequest) -> JsonRpcResponse {
    let params = match &request.params {
        Some(p) => p,
        None => {
            return JsonRpcResponse::invalid_params(
                request.id.clone(),
                "Missing required 'params' field".to_string(),
            );
        }
    };

    let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return JsonRpcResponse::invalid_params(
                request.id.clone(),
                "Missing required parameter 'session_id'".to_string(),
            );
        }
    };

    match service.cancel_session(session_id).await {
        Ok(Some(conversation_id)) => {
            info!(
                "Cancelled session {} mapped to conversation {}",
                session_id, conversation_id
            );
            JsonRpcResponse::success(
                request.id.clone(),
                json!({
                    "cancelled": true,
                    "session_id": session_id,
                    "conversation_id": conversation_id,
                }),
            )
        }
        Ok(None) => JsonRpcResponse::invalid_params(
            request.id.clone(),
            format!("Unknown session id '{session_id}'"),
        ),
        Err(e) => {
            error!("Cancel failed: {}", e);
            JsonRpcResponse::internal_error(request.id.clone(), format!("Cancel failed: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GatewayConfig;

    #[tokio::test]
    async fn test_conversation_prompt_missing_params() -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new(GatewayConfig::default()).await?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "conversation.prompt".to_string(),
            params: None,
            id: Some(json!(1)),
        };

        let result = handle_jsonrpc(State(state), Json(request)).await;

        assert!(result.is_ok());
        let (status, json_response) = result.unwrap();
        assert_eq!(status, StatusCode::OK);

        let response = json_response.0;
        assert_eq!(response.jsonrpc, "2.0");
        let error = response.error.expect("expected validation error");
        assert_eq!(error.code, -32602);
        Ok(())
    }

    #[tokio::test]
    async fn test_conversation_prompt_missing_prompt_field()
    -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new(GatewayConfig::default()).await?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "conversation.prompt".to_string(),
            params: Some(json!({"session_id": "test-session"})),
            id: Some(json!(2)),
        };

        let result = handle_jsonrpc(State(state), Json(request)).await;

        assert!(result.is_ok());
        let (status, json_response) = result.unwrap();
        assert_eq!(status, StatusCode::OK);

        let response = json_response.0;
        assert_eq!(response.jsonrpc, "2.0");
        let error = response.error.expect("expected missing prompt error");
        assert_eq!(error.code, -32602);
        Ok(())
    }

    #[tokio::test]
    async fn test_conversation_prompt_returns_events_field()
    -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new(GatewayConfig::default()).await?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "conversation.prompt".to_string(),
            params: Some(json!({
                "prompt": "create hello world script",
                "session_id": "prompt-session"
            })),
            id: Some(json!(3)),
        };

        let result = handle_jsonrpc(State(state), Json(request)).await;

        assert!(result.is_ok());
        let (status, json_response) = result.unwrap();
        assert_eq!(status, StatusCode::OK);

        let response = json_response.0;
        assert_eq!(response.jsonrpc, "2.0");
        let result_value = response.result.expect("expected prompt result");
        assert!(
            result_value.get("events").is_some(),
            "expected events field in response"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_conversation_status_requires_session_id() -> Result<(), Box<dyn std::error::Error>>
    {
        let state = AppState::new(GatewayConfig::default()).await?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "conversation.status".to_string(),
            params: None,
            id: Some(json!(4)),
        };

        let result = handle_jsonrpc(State(state), Json(request)).await;

        assert!(result.is_ok());
        let (status, json_response) = result.unwrap();
        assert_eq!(status, StatusCode::OK);

        let response = json_response.0;
        assert_eq!(response.jsonrpc, "2.0");
        let error = response.error.expect("expected missing session error");
        assert_eq!(error.code, -32602);
        Ok(())
    }

    #[tokio::test]
    async fn test_conversation_status_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new(GatewayConfig::default()).await?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "conversation.status".to_string(),
            params: Some(json!({"session_id": "missing-session"})),
            id: Some(json!(5)),
        };

        let result = handle_jsonrpc(State(state), Json(request)).await;

        assert!(result.is_ok());
        let (status, json_response) = result.unwrap();
        assert_eq!(status, StatusCode::OK);

        let response = json_response.0;
        assert_eq!(response.jsonrpc, "2.0");
        let result_value = response.result.expect("expected status result");
        assert_eq!(result_value.get("status"), Some(&json!("not_found")));
        Ok(())
    }

    #[tokio::test]
    async fn test_conversation_cancel_unknown_session() -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new(GatewayConfig::default()).await?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "conversation.cancel".to_string(),
            params: Some(json!({"session_id": "unknown"})),
            id: Some(json!(6)),
        };

        let result = handle_jsonrpc(State(state), Json(request)).await;

        assert!(result.is_ok());
        let (status, json_response) = result.unwrap();
        assert_eq!(status, StatusCode::OK);

        let response = json_response.0;
        assert_eq!(response.jsonrpc, "2.0");
        let error = response.error.expect("expected invalid session error");
        assert_eq!(error.code, -32602);
        Ok(())
    }

    #[tokio::test]
    async fn test_unknown_method() -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new(GatewayConfig::default()).await?;
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "unknown_method".to_string(),
            params: None,
            id: Some(json!(7)),
        };

        let result = handle_jsonrpc(State(state), Json(request)).await;

        assert!(result.is_ok());
        let (status, json_response) = result.unwrap();
        assert_eq!(status, StatusCode::OK);

        let response = json_response.0;
        assert_eq!(response.jsonrpc, "2.0");
        let error = response.error.expect("expected method not found error");
        assert_eq!(error.code, -32601);
        let available = error
            .data
            .and_then(|value| value.get("available_methods").cloned())
            .unwrap_or_else(|| json!([]));
        assert_eq!(
            available,
            json!([
                "conversation.prompt",
                "conversation.status",
                "conversation.cancel"
            ])
        );
        Ok(())
    }
}
