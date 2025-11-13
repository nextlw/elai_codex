//! Integration test for CodexService execute_prompt
//!
//! This test validates the real integration between the Gateway and Codex core.

use codex_gateway::services::codex_service::CodexService;

#[tokio::test]
async fn test_execute_prompt_real() -> Result<(), Box<dyn std::error::Error>> {
    // Create CodexService instance (uses real Codex integration)
    let service = CodexService::new().await?;

    // Simple test prompt
    let prompt = "Write a Rust function that adds two numbers.";

    // Execute the prompt with real Codex integration
    let response = service.execute_prompt(prompt, None).await?;

    // Validate response structure
    assert!(
        response.is_object(),
        "Response should be a JSON object, got: {response:?}"
    );

    // Validate required fields
    assert_eq!(
        response.get("type").and_then(|v| v.as_str()),
        Some("ai_response"),
        "Response should have type='ai_response'"
    );

    assert!(
        response.get("conversation_id").is_some(),
        "Response should contain conversation_id"
    );

    assert!(
        response.get("content").is_some(),
        "Response should contain content field"
    );

    assert!(
        response.get("timestamp").is_some(),
        "Response should contain timestamp"
    );

    assert!(
        response.get("events").and_then(|v| v.as_array()).is_some(),
        "Response should contain events array"
    );

    // Validate content (should contain Rust code)
    let content = response
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("");

    assert!(!content.is_empty(), "Response content should not be empty");

    // Check if response likely contains a Rust function
    // (relaxed check since AI output varies)
    let has_rust_indicators =
        content.contains("fn") || content.contains("pub fn") || content.contains("function");

    assert!(
        has_rust_indicators,
        "Response should contain Rust function indicators, got content: {content}"
    );

    println!(
        "✅ Test passed! Response content preview: {}",
        &content.chars().take(100).collect::<String>()
    );

    Ok(())
}

#[tokio::test]
async fn test_execute_prompt_with_session() -> Result<(), Box<dyn std::error::Error>> {
    // Create CodexService instance
    let service = CodexService::new().await?;

    // Execute with a specific session ID
    let session_id = Some("test-session-123");
    let prompt = "Say hello";

    let response = service.execute_prompt(prompt, session_id).await?;

    // Validate response
    assert!(response.is_object());
    assert_eq!(
        response.get("type").and_then(|v| v.as_str()),
        Some("ai_response")
    );

    let content = response
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("");

    assert!(!content.is_empty(), "Content should not be empty");

    println!("✅ Session test passed! Content: {content}");

    Ok(())
}
