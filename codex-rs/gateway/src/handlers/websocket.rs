//! WebSocket handler for real-time exec event streaming
//!
//! This module provides REAL WebSocket streaming of codex-exec events,
//! allowing clients to receive JSONL events in real-time as they occur.
//!
//! ## Architecture
//!
//! ```text
//! WebSocket Connection
//!         ↓
//! {"type": "exec", "prompt": "...", ...}
//!         ↓
//! Background Task (event loop)
//!         ↓
//! EventProcessorWithJsonOutput
//!         ↓
//! Stream ThreadEvents to client
//!         ↓
//! {"type": "thread.started", ...}
//! {"type": "turn.started", ...}
//! {"type": "item.completed", ...}
//! {"type": "turn.completed", ...}
//! ```

use crate::error::GatewayResult;
use crate::state::AppState;
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::response::Response;
use codex_exec::event_processor_with_jsonl_output::EventProcessorWithJsonOutput;
use codex_exec::exec_events::ThreadEvent;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use futures::SinkExt;
use futures::StreamExt;
use futures::stream::SplitSink;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

/// WebSocket request messages from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WebSocketRequest {
    /// Execute a prompt in exec mode (same as POST /exec but streaming)
    Exec {
        prompt: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
        #[serde(default)]
        images: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_schema: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<PathBuf>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    /// Interrupt current execution
    Interrupt { session_id: String },
    /// Ping for keep-alive
    Ping,
}

/// WebSocket response messages to client
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WebSocketResponse {
    /// JSONL event from exec (matches ThreadEvent from codex-exec)
    Event { event: Box<ThreadEvent> },
    /// Acknowledgment of command
    Ack { message: String },
    /// Error message
    Error { message: String },
    /// Pong response to ping
    Pong,
}

/// Handle WebSocket upgrade request
///
/// This is the entry point for WebSocket connections. It upgrades the HTTP
/// connection to a WebSocket connection and starts the message loop.
pub async fn handle_websocket_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> GatewayResult<Response> {
    info!("WebSocket upgrade requested");
    Ok(ws.on_upgrade(|socket| handle_websocket_connection(socket, state)))
}

/// Handle WebSocket connection lifecycle
///
/// Splits the WebSocket into sender and receiver, then enters the main
/// message loop where it processes client requests and streams responses.
async fn handle_websocket_connection(socket: WebSocket, state: AppState) {
    info!("WebSocket connection established");

    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    // Main message loop - process client messages
    while let Some(msg_result) = receiver.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                let text_str = text.to_string();
                debug!("Received WebSocket text message: {}", text_str);
                let sender_clone = Arc::clone(&sender);
                if let Err(e) = handle_text_message(text_str, &state, sender_clone).await {
                    error!("Error handling WebSocket message: {e}");
                    let sender_clone = Arc::clone(&sender);
                    let _ = send_error(sender_clone, format!("Error: {e}")).await;
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket closed by client");
                break;
            }
            Ok(Message::Ping(data)) => {
                debug!("Received ping, sending pong");
                let mut sender_lock = sender.lock().await;
                if sender_lock.send(Message::Pong(data)).await.is_err() {
                    warn!("Failed to send pong");
                    break;
                }
            }
            Ok(Message::Pong(_)) => {
                debug!("Received pong");
            }
            Err(e) => {
                error!("WebSocket error: {e}");
                break;
            }
            _ => {
                debug!("Received other WebSocket message type");
            }
        }
    }

    info!("WebSocket connection closed");
}

/// Handle incoming text messages from WebSocket client
///
/// Parses the JSON message and dispatches to the appropriate handler
/// based on the message type (exec, interrupt, ping).
async fn handle_text_message(
    text: String,
    state: &AppState,
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
) -> anyhow::Result<()> {
    let request: WebSocketRequest = serde_json::from_str(&text)?;

    match request {
        WebSocketRequest::Exec {
            prompt,
            session_id,
            images,
            output_schema,
            cwd,
            model,
        } => {
            handle_exec_request(
                prompt,
                session_id,
                images,
                output_schema,
                cwd,
                model,
                state,
                sender,
            )
            .await
        }
        WebSocketRequest::Interrupt { session_id } => {
            handle_interrupt_request(session_id, state, sender).await
        }
        WebSocketRequest::Ping => {
            let response = WebSocketResponse::Pong;
            let json = serde_json::to_string(&response)?;
            let mut sender_lock = sender.lock().await;
            sender_lock.send(Message::Text(json.into())).await?;
            Ok(())
        }
    }
}

/// Handle exec request via WebSocket (streaming version of POST /exec)
///
/// This is the core streaming implementation. It:
/// 1. Creates/gets conversation
/// 2. Spawns background task with EventProcessorWithJsonOutput
/// 3. Submits Op::UserTurn
/// 4. Streams ThreadEvents to client in real-time
#[allow(clippy::too_many_arguments)]
async fn handle_exec_request(
    prompt: String,
    session_id: Option<String>,
    images: Vec<String>,
    output_schema: Option<Value>,
    cwd: Option<PathBuf>,
    model: Option<String>,
    state: &AppState,
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
) -> anyhow::Result<()> {
    info!(
        "Handling WebSocket exec request: prompt_len={}, session_id={:?}",
        prompt.len(),
        session_id
    );

    // 1. Get or create conversation
    let conversation_id = state
        .codex_service
        .get_or_create_conversation(session_id.as_deref())
        .await?;

    debug!("Using conversation_id: {}", conversation_id);

    // 2. Get conversation from ConversationManager
    let conversation = {
        let manager = state.codex_service.conversation_manager().lock().await;
        manager.get_conversation(conversation_id).await?
    };

    // 3. Prepare UserInputs
    let mut user_inputs: Vec<UserInput> = vec![];

    // Add images
    for img in images {
        if img.starts_with("data:") {
            user_inputs.push(UserInput::Image { image_url: img });
        } else {
            user_inputs.push(UserInput::LocalImage {
                path: PathBuf::from(img),
            });
        }
    }

    // Add text prompt
    user_inputs.push(UserInput::Text { text: prompt });

    // 4. Get config for Op::UserTurn params
    let config = state.codex_service.codex_config();
    let cwd = cwd.unwrap_or_else(|| config.cwd.clone());
    let model = model.unwrap_or_else(|| config.model.clone());

    // 5. Create channel for event streaming
    let (tx, mut rx) = mpsc::unbounded_channel::<ThreadEvent>();

    // 6. Spawn background task to process events using REAL EventProcessorWithJsonOutput
    let conversation_clone = conversation.clone();
    tokio::spawn(async move {
        let mut processor = EventProcessorWithJsonOutput::new(None);

        loop {
            match conversation_clone.next_event().await {
                Ok(event) => {
                    debug!("WebSocket: Processing event: {:?}", event.msg);

                    // Use REAL EventProcessorWithJsonOutput
                    let thread_events = processor.collect_thread_events(&event);
                    for te in thread_events {
                        if tx.send(te).is_err() {
                            error!("WebSocket: Failed to send event to channel (receiver dropped)");
                            break;
                        }
                    }

                    // Check for terminal events
                    if matches!(
                        event.msg,
                        EventMsg::TaskComplete(_) | EventMsg::Error(_) | EventMsg::TurnAborted(_)
                    ) {
                        debug!("WebSocket: Terminal event received, stopping event loop");
                        break;
                    }
                }
                Err(e) => {
                    error!("WebSocket: Error receiving event: {e}");
                    break;
                }
            }
        }
    });

    // 7. Submit Op::UserTurn
    info!(
        "WebSocket: Submitting user turn with model={}, cwd={:?}",
        model, cwd
    );
    conversation
        .submit(Op::UserTurn {
            items: user_inputs,
            cwd,
            approval_policy: config.approval_policy,
            sandbox_policy: config.sandbox_policy.clone(),
            model,
            effort: config.model_reasoning_effort,
            summary: config.model_reasoning_summary,
            final_output_json_schema: output_schema,
        })
        .await?;

    // 8. Stream events to client in real-time
    while let Some(thread_event) = rx.recv().await {
        let response = WebSocketResponse::Event {
            event: Box::new(thread_event),
        };
        let json = serde_json::to_string(&response)?;

        let mut sender_lock = sender.lock().await;
        if sender_lock.send(Message::Text(json.into())).await.is_err() {
            warn!("WebSocket: Failed to send event to client (connection closed)");
            break;
        }
    }

    info!(
        "WebSocket: Exec completed for conversation_id={}",
        conversation_id
    );

    Ok(())
}

/// Handle interrupt request via WebSocket
///
/// Submits Op::Interrupt to the conversation to stop execution.
async fn handle_interrupt_request(
    session_id: String,
    state: &AppState,
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
) -> anyhow::Result<()> {
    info!("WebSocket: Interrupt requested for session: {}", session_id);

    // Get conversation ID from session ID
    let conversation_id = {
        let conversations = state.codex_service.active_conversations().lock().await;
        conversations
            .get(&session_id)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Session not found: {session_id}"))?
    };

    // Get conversation
    let conversation = {
        let manager = state.codex_service.conversation_manager().lock().await;
        manager.get_conversation(conversation_id).await?
    };

    // Submit interrupt
    conversation.submit(Op::Interrupt).await?;

    // Send acknowledgment
    let response = WebSocketResponse::Ack {
        message: format!("Interrupt submitted for session {session_id}"),
    };
    let json = serde_json::to_string(&response)?;

    let mut sender_lock = sender.lock().await;
    sender_lock.send(Message::Text(json.into())).await?;

    info!(
        "WebSocket: Interrupt acknowledged for session {}",
        session_id
    );

    Ok(())
}

/// Send error message to WebSocket client
async fn send_error(
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    message: String,
) -> Result<(), axum::Error> {
    let response = WebSocketResponse::Error { message };
    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());

    let mut sender_lock = sender.lock().await;
    sender_lock.send(Message::Text(json.into())).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_request_exec_deserialization() {
        let json = r#"{"type":"exec","prompt":"test","session_id":"123"}"#;
        let request: WebSocketRequest = serde_json::from_str(json).unwrap();

        match request {
            WebSocketRequest::Exec {
                prompt, session_id, ..
            } => {
                assert_eq!(prompt, "test");
                assert_eq!(session_id, Some("123".to_string()));
            }
            _ => panic!("Expected Exec variant"),
        }
    }

    #[test]
    fn test_websocket_request_ping_deserialization() {
        let json = r#"{"type":"ping"}"#;
        let request: WebSocketRequest = serde_json::from_str(json).unwrap();

        match request {
            WebSocketRequest::Ping => {}
            _ => panic!("Expected Ping variant"),
        }
    }

    #[test]
    fn test_websocket_response_event_serialization() {
        use codex_exec::exec_events::*;

        let thread_event = ThreadEvent::TurnCompleted(TurnCompletedEvent {
            usage: Default::default(),
        });

        let response = WebSocketResponse::Event {
            event: Box::new(thread_event),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"type\":\"event\""));
    }

    #[test]
    fn test_websocket_response_ack_serialization() {
        let response = WebSocketResponse::Ack {
            message: "OK".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"type\":\"ack\""));
        assert!(json.contains("\"message\":\"OK\""));
    }
}
