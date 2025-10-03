//! Tipos auxiliares para API do wrapper Cloud Run

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct ExecRequest {
    pub prompt: String,
    pub timeout_ms: Option<u64>,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub recommended_endpoint: Option<String>,
    pub status: u16,
}
