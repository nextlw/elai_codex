//! WebSocket handler

use crate::error::GatewayResult;
use crate::state::AppState;
use axum::extract::State;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::Response;

/// WebSocket upgrade endpoint
///
/// Upgrades HTTP connections to WebSocket for real-time communication.
/// This is a placeholder implementation that echoes messages back to the client.
///
/// Future iterations will implement:
/// - Protocol-aware message routing
/// - Session management
/// - Service multiplexing
/// - Authentication/authorization
pub async fn handle_websocket_upgrade(
    ws: WebSocketUpgrade,
    State(_state): State<AppState>,
) -> GatewayResult<Response> {
    tracing::info!("WebSocket upgrade requested");

    let response = ws.on_upgrade(handle_websocket_connection);

    Ok(response)
}

/// Handle individual WebSocket connections
async fn handle_websocket_connection(mut socket: axum::extract::ws::WebSocket) {
    tracing::info!("WebSocket connection established");

    loop {
        use axum::extract::ws::Message;

        match socket.recv().await {
            Some(Ok(message)) => {
                match message {
                    Message::Text(text) => {
                        tracing::debug!("Received text message: {}", text);

                        // Echo the message back (placeholder behavior)
                        let response = format!("Echo: {text}");
                        if socket.send(Message::Text(response.into())).await.is_err() {
                            tracing::warn!("Failed to send WebSocket response");
                            break;
                        }
                    }
                    Message::Binary(data) => {
                        tracing::debug!("Received binary message: {} bytes", data.len());

                        // Echo binary data back
                        if socket.send(Message::Binary(data)).await.is_err() {
                            tracing::warn!("Failed to send WebSocket binary response");
                            break;
                        }
                    }
                    Message::Close(_) => {
                        tracing::info!("WebSocket connection closed by client");
                        break;
                    }
                    Message::Ping(data) => {
                        tracing::debug!("Received ping, sending pong");
                        if socket.send(Message::Pong(data)).await.is_err() {
                            tracing::warn!("Failed to send pong response");
                            break;
                        }
                    }
                    Message::Pong(_) => {
                        tracing::debug!("Received pong");
                    }
                }
            }
            Some(Err(e)) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            None => {
                tracing::info!("WebSocket connection terminated");
                break;
            }
        }
    }

    tracing::info!("WebSocket connection handler finished");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GatewayConfig;

    #[tokio::test]
    async fn test_websocket_handler_compiles() -> Result<(), Box<dyn std::error::Error>> {
        // This test ensures the handler compiles correctly
        // Full WebSocket testing requires more complex setup
        let _state = AppState::new(GatewayConfig::default()).await?;
        // Actual WebSocket testing would require test client setup
        Ok(())
    }
}
