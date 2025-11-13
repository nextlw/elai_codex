//! Configuration types for the Codex Gateway

use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;

/// Gateway configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Host to bind the server to
    pub host: String,

    /// Port to bind the server to
    pub port: u16,

    /// Timeout configurations
    pub timeouts: TimeoutConfig,

    /// Maximum number of concurrent connections
    pub max_connections: usize,

    /// WebSocket configuration
    pub websocket: WebSocketConfig,

    /// Request body size limits configuration
    pub body_limits: BodyLimitsConfig,
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Request timeout duration
    pub request_timeout: Duration,

    /// Keep-alive timeout duration
    pub keep_alive_timeout: Duration,

    /// WebSocket ping interval
    pub websocket_ping_interval: Duration,

    /// WebSocket connection timeout
    pub websocket_timeout: Duration,
}

/// WebSocket-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Maximum message size in bytes
    pub max_message_size: usize,

    /// Maximum frame size in bytes
    pub max_frame_size: usize,

    /// Enable compression
    pub enable_compression: bool,

    /// Maximum number of concurrent WebSocket connections
    pub max_connections: usize,
}

/// Request body size limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyLimitsConfig {
    /// Global default body size limit in bytes
    pub default_limit: usize,

    /// JSON-RPC specific body size limit in bytes
    pub jsonrpc_limit: usize,

    /// Webhook specific body size limit in bytes
    pub webhook_limit: usize,

    /// Health check body size limit in bytes (usually very small)
    pub health_limit: usize,

    /// Whether to enable body size limits (can be disabled for development)
    pub enabled: bool,
    pub(crate) websocket_limit: usize,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            timeouts: TimeoutConfig::default(),
            max_connections: 10000,
            websocket: WebSocketConfig::default(),
            body_limits: BodyLimitsConfig::default(),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(30),
            keep_alive_timeout: Duration::from_secs(60),
            websocket_ping_interval: Duration::from_secs(30),
            websocket_timeout: Duration::from_secs(300),
        }
    }
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_message_size: 64 * 1024 * 1024, // 64MB
            max_frame_size: 16 * 1024 * 1024,   // 16MB
            enable_compression: true,
            // Limite baseado em padrões de mercado e capacidade do servidor
            // nginx default: 1024, cloudflare: 10000, optamos por um meio termo robusto
            max_connections: 5000,
        }
    }
}

impl Default for BodyLimitsConfig {
    fn default() -> Self {
        Self {
            // Global default: 2MB - alinhado com padrão Axum, balanceamento segurança/funcionalidade
            default_limit: 2 * 1024 * 1024,

            // JSON-RPC: 1MB - suficiente para comandos CLI complexos
            jsonrpc_limit: 1024 * 1024,

            // Webhooks: 10MB - suporte a payloads de integração robustos (ex: GitHub webhooks com diffs grandes)
            webhook_limit: 10 * 1024 * 1024,

            // Health: 1KB - endpoints de health são mínimos
            health_limit: 1024,

            // Websocket: 1MB - suficiente para payloads de integração robustos (ex: GitHub webhooks com diffs grandes)
            websocket_limit: 1024,

            // Habilitado por padrão para produção
            enabled: true,
        }
    }
}

impl BodyLimitsConfig {
    /// Create body limits config from environment variables
    pub fn from_env() -> Self {
        let parse_size = |env_var: &str, default: usize| -> usize {
            std::env::var(env_var)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        };

        let enabled = std::env::var("GATEWAY_BODY_LIMITS_ENABLED")
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true);

        Self {
            default_limit: parse_size("GATEWAY_BODY_LIMIT_DEFAULT", 2 * 1024 * 1024),
            jsonrpc_limit: parse_size("GATEWAY_BODY_LIMIT_JSONRPC", 1024 * 1024),
            webhook_limit: parse_size("GATEWAY_BODY_LIMIT_WEBHOOK", 10 * 1024 * 1024),
            health_limit: parse_size("GATEWAY_BODY_LIMIT_HEALTH", 1024),
            websocket_limit: parse_size("GATEWAY_BODY_LIMIT_WEBSOCKET", 1024 * 1024),
            enabled,
        }
    }

    /// Get the appropriate limit for a given endpoint path
    pub fn get_limit_for_path(&self, path: &str) -> usize {
        if !self.enabled {
            return usize::MAX; // No limit when disabled
        }

        match path {
            p if p.starts_with("/health") => self.health_limit,
            p if p.starts_with("/jsonrpc") || p.starts_with("/rpc") => self.jsonrpc_limit,
            p if p.starts_with("/webhook") || p.starts_with("/hook") => self.webhook_limit,
            _ => self.default_limit,
        }
    }
}

impl GatewayConfig {
    /// Create a new config from environment variables
    pub fn from_env() -> Self {
        let host = std::env::var("GATEWAY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("GATEWAY_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080);

        // Body limits configuration from environment
        let body_limits = BodyLimitsConfig::from_env();

        // WebSocket configuration from environment
        let mut websocket = WebSocketConfig::default();
        if let Ok(max_conn_str) = std::env::var("GATEWAY_WEBSOCKET_MAX_CONNECTIONS")
            && let Ok(max_conn) = max_conn_str.parse::<usize>()
        {
            websocket.max_connections = max_conn;
        }

        Self {
            host,
            port,
            body_limits,
            websocket,
            ..Default::default()
        }
    }

    /// Get the full bind address
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
