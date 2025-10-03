//! Handlers dos endpoints da API do wrapper Cloud Run

use crate::process::run_codex_app_server_stream;
use crate::types::ErrorResponse;
use crate::types::ExecRequest;
use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Sse;

/// Handler para POST /api/v1/exec/stream (SSE)
pub async fn exec_stream_handler(
    Json(req): Json<ExecRequest>,
) -> Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>
{
    let stream = run_codex_app_server_stream(req).await;
    Sse::new(stream)
}

/// Handler para POST /api/v1/exec (legacy, retorna 422)
pub async fn exec_legacy_handler(Json(_req): Json<ExecRequest>) -> impl IntoResponse {
    let err = ErrorResponse {
        error: "Use /api/v1/exec/stream endpoint for real-time execution".to_string(),
        recommended_endpoint: Some("/api/v1/exec/stream".to_string()),
        status: 422,
    };
    (StatusCode::UNPROCESSABLE_ENTITY, axum::Json(err))
}
