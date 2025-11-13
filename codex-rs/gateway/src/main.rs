//! Main entry point for the Codex Gateway server

use codex_gateway::config::BodyLimitsConfig;
use codex_gateway::config::GatewayConfig;
use codex_gateway::error::GatewayError;
use codex_gateway::error::GatewayResult;
use codex_gateway::router::create_router;
use codex_gateway::state::AppState;
use std::env;
use std::net::SocketAddr;
use std::process;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::error;
use tracing::info;
use tracing::warn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Main entry point
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        error!("Server failed to start: {}", e);
        process::exit(1);
    }
}

/// Main server execution function
async fn run() -> GatewayResult<()> {
    // Initialize tracing subscriber with structured logging
    init_tracing()?;

    info!("Starting Codex Gateway server");

    // Load configuration
    let config = load_config()?;
    info!("Configuration loaded: {:?}", config);

    // Create application state
    let state = AppState::new(config.clone()).await?;

    // Create router with all routes and middleware
    let app = create_router(state)?;

    // Parse server address from config
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Server will bind to: {}", addr);

    // Create TCP listener
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| GatewayError::ServerStart(format!("Failed to bind to {addr}: {e}")))?;

    info!("Server listening on {}", addr);

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| GatewayError::ServerStart(format!("Server error: {e}")))?;

    info!("Server shutdown complete");
    Ok(())
}

/// Initialize structured logging with tracing subscriber
fn init_tracing() -> GatewayResult<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "codex_gateway=info,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Tracing initialized");
    Ok(())
}

/// Load configuration from environment variables
fn load_config() -> GatewayResult<GatewayConfig> {
    // Get port from environment variable or use default
    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let mut config = GatewayConfig {
        port,
        ..Default::default()
    };

    // Override other config values from environment if available
    if let Ok(timeout_str) = env::var("REQUEST_TIMEOUT_SECS") {
        if let Ok(timeout_secs) = timeout_str.parse::<u64>() {
            config.timeouts.request_timeout = std::time::Duration::from_secs(timeout_secs);
        } else {
            warn!(
                "Invalid REQUEST_TIMEOUT_SECS value: {}, using default",
                timeout_str
            );
        }
    }

    // Body size limits are fully implemented in router middleware with endpoint-specific limits
    // Configuration is handled via BodyLimitsConfig and environment variables:
    // - GATEWAY_BODY_LIMIT_DEFAULT (default: 2MB)
    // - GATEWAY_BODY_LIMIT_JSONRPC (default: 1MB)
    // - GATEWAY_BODY_LIMIT_WEBHOOK (default: 10MB)
    // - GATEWAY_BODY_LIMIT_HEALTH (default: 1KB)
    // - GATEWAY_BODY_LIMITS_ENABLED (default: true)

    // Override body limits configuration from environment if available
    config.body_limits = BodyLimitsConfig::from_env();

    info!(
        "Body size limits configured: default={}KB, jsonrpc={}KB, webhook={}KB, health={}KB, enabled={}",
        config.body_limits.default_limit / 1024,
        config.body_limits.jsonrpc_limit / 1024,
        config.body_limits.webhook_limit / 1024,
        config.body_limits.health_limit / 1024,
        config.body_limits.enabled
    );

    Ok(config)
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        match signal::ctrl_c().await {
            Ok(_) => info!("Received Ctrl+C, initiating graceful shutdown"),
            Err(e) => {
                error!("Failed to install Ctrl+C handler: {}", e);
            }
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                if stream.recv().await.is_some() {
                    info!("Received SIGTERM, initiating graceful shutdown");
                }
            }
            Err(e) => {
                error!("Failed to install SIGTERM handler: {}", e);
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_defaults() {
        let config = load_config().unwrap();
        assert_eq!(config.port, 8080);
        assert!(config.timeouts.request_timeout.as_secs() > 0);
    }

    #[test]
    fn test_load_config_with_env() {
        unsafe {
            env::set_var("PORT", "3000");
            env::set_var("REQUEST_TIMEOUT_SECS", "60");
        }

        let config = load_config().unwrap();
        assert_eq!(config.port, 3000);
        assert_eq!(config.timeouts.request_timeout.as_secs(), 60);

        // Clean up
        unsafe {
            env::remove_var("PORT");
            env::remove_var("REQUEST_TIMEOUT_SECS");
        }
    }
}
