//! Middleware de autenticação para o wrapper Cloud Run

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use std::env;

/// Middleware que valida API Key via header Authorization: Bearer <token>
pub async fn auth_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    // Se GATEWAY_API_KEY não estiver definida, permite acesso (modo desenvolvimento)
    let required_key = match env::var("GATEWAY_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            tracing::warn!("GATEWAY_API_KEY not set - authentication disabled (dev mode)");
            return Ok(next.run(request).await);
        }
    };

    // Extrai o token do header Authorization
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = header.trim_start_matches("Bearer ");
            if token == required_key {
                Ok(next.run(request).await)
            } else {
                tracing::warn!("Invalid API key provided");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => {
            tracing::warn!("Missing or malformed Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
