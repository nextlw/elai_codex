//! Codex Gateway - HTTP/WebSocket Gateway for Codex Services
//!
//! This crate provides a cloud-native gateway that acts as a complete wrapper
//! for all Codex CLI services, maintaining 100% compatibility with the existing protocol.

pub mod config;
pub mod error;
pub mod handlers;
pub mod router;
pub mod services;
pub mod state;

pub use config::GatewayConfig;
pub use config::TimeoutConfig;
pub use config::WebSocketConfig;
pub use error::GatewayError;
pub use error::GatewayResult;
pub use handlers::*;
pub use state::AppState;
