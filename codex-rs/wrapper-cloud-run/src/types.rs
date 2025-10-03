//! Tipos auxiliares para API do wrapper Cloud Run

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ExecRequest {
    pub prompt: String,
    pub timeout_ms: Option<u64>,
    pub session_id: Option<String>,
    pub model: Option<String>,  // Campo opcional para especificar o modelo
    pub allow_network: Option<bool>, // Permitir acesso à rede
    pub allow_file_operations: Option<bool>, // Permitir operações de arquivo
    pub upload_files: Option<bool>, // Fazer upload automático de arquivos criados
    pub approval_policy: Option<String>, // Política de aprovação: "auto", "never", "ask"
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub recommended_endpoint: Option<String>,
    pub status: u16,
}
