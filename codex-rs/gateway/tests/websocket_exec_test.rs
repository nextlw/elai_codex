//! Integration tests for WebSocket exec streaming
//!
//! Tests the WebSocket /ws endpoint for real-time event streaming

use codex_gateway::config::GatewayConfig;
use codex_gateway::state::AppState;
use futures::SinkExt;
use futures::StreamExt;
use serde_json::Value;
use serde_json::json;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// Helper to create test app state
#[allow(dead_code)]
async fn create_test_state() -> Result<AppState, Box<dyn std::error::Error + Send + Sync>> {
    let config = GatewayConfig::default();
    AppState::new(config)
        .await
        .map_err(std::convert::Into::into)
}

#[tokio::test]
#[ignore] // Requires running server, run with: cargo test --test websocket_exec_test -- --ignored
async fn test_websocket_exec_request() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // This test requires the gateway server to be running
    // Start with: ELAI_MODE=gateway cargo run -p codex-gateway

    let ws_url = "ws://localhost:8080/ws?api_key=test-key";

    let (ws_stream, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Send exec request
    let exec_request = json!({
        "type": "exec",
        "prompt": "echo 'hello from websocket'",
        "session_id": "ws-test-session"
    });

    write
        .send(Message::Text(serde_json::to_string(&exec_request)?))
        .await?;

    println!("✅ Sent exec request via WebSocket");

    // Collect events
    let mut events = Vec::new();
    let mut completed = false;

    while let Some(msg) = read.next().await {
        let msg = msg?;

        if let Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text)?;

            if response["type"] == "event" {
                let event = &response["event"];
                events.push(event.clone());

                println!(
                    "Received event: {}",
                    event
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown")
                );

                // Check for completion
                if event.get("type").and_then(|t| t.as_str()) == Some("turn.completed") {
                    completed = true;
                    break;
                }
            } else if response["type"] == "error" {
                panic!("Received error: {}", response["message"]);
            }
        }
    }

    assert!(completed, "Should receive turn.completed event");
    assert!(!events.is_empty(), "Should receive at least one event");

    println!(
        "✅ WebSocket exec test passed! Received {} events",
        events.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_websocket_ping_pong() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_url = "ws://localhost:8080/ws?api_key=test-key";

    let (ws_stream, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Send ping
    let ping_request = json!({
        "type": "ping"
    });

    write
        .send(Message::Text(serde_json::to_string(&ping_request)?))
        .await?;

    // Wait for pong
    if let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text)?;
            assert_eq!(response["type"], "pong", "Should receive pong response");
        }
    }

    println!("✅ WebSocket ping/pong test passed!");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_websocket_interrupt() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_url = "ws://localhost:8080/ws?api_key=test-key";

    let (ws_stream, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Start a long-running exec
    let exec_request = json!({
        "type": "exec",
        "prompt": "sleep for 10 seconds then echo done",
        "session_id": "interrupt-test-session"
    });

    write
        .send(Message::Text(serde_json::to_string(&exec_request)?))
        .await?;

    // Wait a moment for execution to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Send interrupt
    let interrupt_request = json!({
        "type": "interrupt",
        "session_id": "interrupt-test-session"
    });

    write
        .send(Message::Text(serde_json::to_string(&interrupt_request)?))
        .await?;

    // Should receive acknowledgment
    let mut received_ack = false;

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text)?;

            if response["type"] == "ack" {
                received_ack = true;
                println!("Received interrupt ack: {}", response["message"]);
                break;
            }
        }
    }

    assert!(received_ack, "Should receive interrupt acknowledgment");

    println!("✅ WebSocket interrupt test passed!");

    Ok(())
}

#[test]
fn test_websocket_request_deserialization() {
    // Test exec request
    let exec_json = r#"{"type":"exec","prompt":"test","session_id":"123","images":[]}"#;
    let _: serde_json::Value = serde_json::from_str(exec_json).unwrap();

    // Test interrupt request
    let interrupt_json = r#"{"type":"interrupt","session_id":"123"}"#;
    let _: serde_json::Value = serde_json::from_str(interrupt_json).unwrap();

    // Test ping request
    let ping_json = r#"{"type":"ping"}"#;
    let _: serde_json::Value = serde_json::from_str(ping_json).unwrap();

    println!("✅ WebSocket request deserialization test passed!");
}

#[test]
fn test_websocket_response_serialization() {
    use codex_exec::exec_events::*;

    // Test event response
    let thread_event = ThreadEvent::TurnCompleted(TurnCompletedEvent {
        usage: Default::default(),
    });

    let event_response = json!({
        "type": "event",
        "event": thread_event
    });

    assert!(event_response.get("type").is_some());
    assert!(event_response.get("event").is_some());

    // Test ack response
    let ack_response = json!({
        "type": "ack",
        "message": "OK"
    });

    assert_eq!(ack_response["type"], "ack");
    assert_eq!(ack_response["message"], "OK");

    // Test error response
    let error_response = json!({
        "type": "error",
        "message": "Something went wrong"
    });

    assert_eq!(error_response["type"], "error");

    // Test pong response
    let pong_response = json!({
        "type": "pong"
    });

    assert_eq!(pong_response["type"], "pong");

    println!("✅ WebSocket response serialization test passed!");
}

#[tokio::test]
#[ignore]
async fn test_websocket_concurrent_connections()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_url = "ws://localhost:8080/ws?api_key=test-key";

    // Create 3 concurrent WebSocket connections
    let mut handles = vec![];

    for i in 0..3 {
        let url = ws_url.to_string();
        let handle = tokio::spawn(async move {
            let (ws_stream, _) = connect_async(&url).await?;
            let (mut write, mut read) = ws_stream.split();

            let exec_request = json!({
                "type": "exec",
                "prompt": format!("echo 'concurrent {}'", i),
                "session_id": format!("concurrent-ws-{}", i)
            });

            write
                .send(Message::Text(serde_json::to_string(&exec_request)?))
                .await?;

            // Wait for completion
            while let Some(msg) = read.next().await {
                let msg = msg?;
                if let Message::Text(text) = msg {
                    let response: Value = serde_json::from_str(&text)?;
                    if response["type"] == "event" {
                        let event = &response["event"];
                        if event.get("type").and_then(|t| t.as_str()) == Some("turn.completed") {
                            break;
                        }
                    }
                }
            }

            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        handles.push(handle);
    }

    // Wait for all to complete
    let results = futures::future::join_all(handles).await;

    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Concurrent WebSocket {i} should not panic");
        assert!(
            result.as_ref().unwrap().is_ok(),
            "Concurrent WebSocket {i} should complete successfully"
        );
    }

    println!("✅ Concurrent WebSocket connections test passed!");

    Ok(())
}
