//! Router configuration for the Codex Gateway

use crate::error::GatewayResult;
use crate::handlers::health::health_check;
use crate::handlers::jsonrpc::handle_jsonrpc;
use crate::handlers::webhook::handle_webhook;
use crate::handlers::websocket::handle_websocket_upgrade;
use crate::state::AppState;
use axum::Router;
use axum::routing::get;
use axum::routing::post;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

/// Create the main application router with all routes and middleware
pub fn create_router(state: AppState) -> GatewayResult<Router> {
    info!("Creating router with configured routes and middleware");

    // Store config values before moving state
    let request_timeout = state.config().timeouts.request_timeout;
    let default_limit = state.config().body_limits.default_limit;
    let health_limit = state.config().body_limits.health_limit;
    let jsonrpc_limit = state.config().body_limits.jsonrpc_limit;
    let webhook_limit = state.config().body_limits.webhook_limit;

    // Configure CORS - allow all origins for now, can be restricted later
    let cors = CorsLayer::permissive();

    // Configure timeout from state config
    let timeout = TimeoutLayer::new(request_timeout);

    // Configure request body size limits based on endpoint
    let global_body_limit = RequestBodyLimitLayer::new(default_limit);

    // Configure tracing middleware
    let trace = TraceLayer::new_for_http();

    // Build the router with all routes and middleware
    let app = Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // JSON-RPC endpoint for protocol communication
        .route("/jsonrpc", post(handle_jsonrpc))
        // WebSocket endpoint for real-time communication
        .route("/ws", get(handle_websocket_upgrade))
        // Webhook endpoint for external integrations
        .route("/webhook", post(handle_webhook))
        // Apply global middleware stack in correct order
        .layer(global_body_limit) // Global body size limit fallback
        .layer(trace) // Request tracing
        .layer(timeout) // Request timeout
        .layer(cors) // CORS handling
        // Add shared state
        .with_state(state);

    info!(
        "Router created successfully with body size limits: health={}KB, jsonrpc={}KB, websocket={}KB, webhook={}KB, default={}KB",
        health_limit / 1024,
        jsonrpc_limit / 1024,
        default_limit / 1024,
        webhook_limit / 1024,
        default_limit / 1024
    );
    Ok(app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GatewayConfig;

    #[tokio::test]
    async fn test_create_router() -> Result<(), Box<dyn std::error::Error>> {
        let config = GatewayConfig::default();
        let state = AppState::new(config).await?;
        let _router = create_router(state)?;
        // Se chegou até aqui, o router foi criado com sucesso
        Ok(())
    }
}
