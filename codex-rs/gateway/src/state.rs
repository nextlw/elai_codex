//! Shared application state for the Codex Gateway

use crate::config::GatewayConfig;
use crate::error::GatewayError;
use crate::services::CodexService;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::runtime::Runtime;

/// Shared application state passed to all handlers
#[derive(Debug, Clone)]
pub struct AppState {
    /// Gateway configuration
    pub config: Arc<GatewayConfig>,
    /// Codex service for processing AI requests
    pub codex_service: Arc<CodexService>,
    // Add more shared state here as needed in future iterations
    // Examples:
    // - Database connections
    // - Redis connections
    // - Service discovery clients
    // - Metrics collectors
    // - Authentication services
}

impl AppState {
    /// Create a new AppState with the given configuration
    pub async fn new(config: GatewayConfig) -> Result<Self, GatewayError> {
        let codex_service = CodexService::new().await?;
        //                                            ^ propaga erro ao invés de panic
        Ok(Self {
            config: Arc::new(config),
            codex_service: Arc::new(codex_service),
        })
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }
}

impl Default for AppState {
    fn default() -> Self {
        if let Ok(handle) = Handle::try_current() {
            handle
                .block_on(AppState::new(GatewayConfig::default()))
                .unwrap_or_else(|err| {
                    panic!("Failed to initialize AppState with default config: {err}")
                })
        } else {
            Runtime::new()
                .unwrap_or_else(|err| {
                    panic!("Failed to create Tokio runtime for AppState::default: {err}")
                })
                .block_on(AppState::new(GatewayConfig::default()))
                .unwrap_or_else(|err| {
                    panic!("Failed to initialize AppState with default config: {err}")
                })
        }
    }
}
