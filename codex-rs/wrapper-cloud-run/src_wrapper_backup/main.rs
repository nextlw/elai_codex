mod api;
mod auth;
mod process;
mod types;

use axum::middleware;
use axum::routing::get;
use axum::routing::post;
use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Carrega variáveis de ambiente do .env (se existir)
    // Ignora erro se .env não existir (útil para produção)
    let _ = dotenvy::dotenv();

    // Inicializa o tracing para logs estruturados
    tracing_subscriber::fmt::init();

    // Log de inicialização com status das credenciais
    if std::env::var("OPENAI_API_KEY").is_ok() {
        tracing::info!("OPENAI_API_KEY detectada");
    }
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        tracing::info!("ANTHROPIC_API_KEY detectada");
    }
    if std::env::var("GATEWAY_API_KEY").is_ok() {
        tracing::info!("GATEWAY_API_KEY configurada - autenticação habilitada");
    } else {
        tracing::warn!("GATEWAY_API_KEY não configurada - modo desenvolvimento (sem autenticação)");
    }

    // Roteador com autenticação
    let protected_routes = Router::new()
        .route("/api/v1/exec/stream", post(api::exec_stream_handler))
        .route("/api/v1/exec", post(api::exec_legacy_handler))
        .layer(middleware::from_fn(auth::auth_middleware));

    // Roteador principal (health sem autenticação)
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .merge(protected_routes);

    // Endereço padrão para Cloud Run
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Iniciando servidor Axum em {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
