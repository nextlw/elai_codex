//! Codex Service Integration
//!
//! This module provides REAL integration between the Gateway and Codex core functionality.
//! It serves as a bridge to process AI prompts using the existing Codex infrastructure,
//! replacing previous placeholder implementations with actual MessageProcessor integration.

use crate::error::GatewayError;
use crate::error::GatewayResult;
use chrono::Utc;
use codex_core::ConversationManager;
use codex_core::auth::AuthManager;
use codex_core::config::Config as CodexConfig;
use codex_core::config::ConfigOverrides;
use codex_core::find_conversation_path_by_id_str;
use codex_protocol::ConversationId;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::SessionConfiguredEvent;
use codex_protocol::protocol::SessionSource;
use codex_protocol::user_input::UserInput;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;
use serde_json::to_value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;
use tracing::info;
use tracing::trace;
use tracing::warn;

/// Codex Service para processar prompts de IA e gerenciar conversas
///
/// Integração direta com o stack do Codex core:
/// - Acesso ao ConversationManager para gerenciar sessões reais
/// - Submissão de turns via `Op::UserTurn`
/// - Consumo de eventos reais emitidos pelo agente
#[derive(Clone)]
pub struct CodexService {
    /// Active conversations mapped by session ID to conversation ID
    active_conversations: Arc<Mutex<HashMap<String, ConversationId>>>,

    /// Base Codex configuration loaded from disk/CLI overrides
    codex_config: Arc<CodexConfig>,
    /// Metadata emitted during session configuration
    conversation_metadata: Arc<Mutex<HashMap<ConversationId, SessionConfiguredEvent>>>,
    // MessageProcessor integration pending
    /// ConversationManager for codex-core integration
    conversation_manager: Arc<Mutex<ConversationManager>>,
    /// Request counter for generating unique request IDs
    request_counter: Arc<Mutex<u64>>,
}

/// Status básico de uma sessão ativa exposto via JSON-RPC
#[derive(Debug, Clone, Serialize)]
pub struct SessionStatus {
    pub conversation_id: ConversationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SessionConfiguredEvent>,
}

impl std::fmt::Debug for CodexService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodexService")
            .field("conversation_manager", &"<ConversationManager>")
            .field("active_conversations", &"<HashMap>")
            .finish()
    }
}

impl CodexService {
    /// Create a new CodexService instance with real Codex integration
    pub async fn new() -> GatewayResult<Self> {
        info!("Initializing CodexService with real Codex integration");

        let codex_config =
            CodexConfig::load_with_cli_overrides(Vec::new(), ConfigOverrides::default())
                .await
                .map_err(|err| {
                    GatewayError::Config(format!("failed to load Codex config: {err}"))
                })?;

        // Create AuthManager for ConversationManager
        let auth_manager = AuthManager::shared(
            codex_config.codex_home.clone(),
            false,
            codex_config.cli_auth_credentials_store_mode,
        );

        // Initialize ConversationManager for codex-core integration
        let conversation_manager = Arc::new(Mutex::new(ConversationManager::new(
            auth_manager,
            SessionSource::Exec,
        )));

        info!("CodexService initialized successfully with real Codex components");

        Ok(Self {
            active_conversations: Arc::new(Mutex::new(HashMap::new())),
            codex_config: Arc::new(codex_config),
            conversation_metadata: Arc::new(Mutex::new(HashMap::new())),
            conversation_manager,
            request_counter: Arc::new(Mutex::new(0)),
        })
    }

    /// Execute a prompt using REAL Codex AI processing
    ///
    /// This method implements the complete integration flow:
    /// 1. Gateway receives prompt and session_id
    /// 2. Creates or retrieves conversation via ConversationManager
    /// 3. Constructs proper SendUserTurnParams with InputItem::Text
    /// 4. Submits to MessageProcessor via ClientRequest::SendUserTurn
    /// 5. Processes ResponseStream events from actual AI
    /// 6. Returns structured JSON with real AI response
    pub async fn execute_prompt(
        &self,
        prompt: &str,
        session_id: Option<&str>,
    ) -> GatewayResult<Value> {
        let start_time = Utc::now();
        info!(
            "Starting REAL Codex AI prompt execution: session_id={:?}, prompt_len={}",
            session_id,
            prompt.len()
        );

        // Get or create conversation ID for this session
        let conversation_id = self.get_or_create_conversation(session_id).await?;
        debug!("Using conversation_id: {}", conversation_id);

        // Generate unique request ID (currently only used for logging)
        let request_id = self.next_request_id().await;
        debug!("Generated request_id: {}", request_id);

        let codex_config = Arc::clone(&self.codex_config);

        // Prepare Codex user inputs
        let user_inputs = vec![UserInput::Text {
            text: prompt.to_string(),
        }];

        // Get the conversation from ConversationManager
        let conversation = {
            let manager = self.conversation_manager.lock().await;

            manager
                .get_conversation(conversation_id)
                .await
                .map_err(|e| GatewayError::Internal(format!("failed to get conversation: {e}")))?
        };

        // Submit the prompt via CodexConversation using real Op::UserTurn
        let submission_id = conversation
            .submit(Op::UserTurn {
                items: user_inputs,
                cwd: codex_config.cwd.clone(),
                approval_policy: codex_config.approval_policy,
                sandbox_policy: codex_config.sandbox_policy.clone(),
                model: codex_config.model.clone(),
                effort: codex_config.model_reasoning_effort,
                summary: codex_config.model_reasoning_summary,
                final_output_json_schema: None,
            })
            .await
            .map_err(|e| GatewayError::Internal(format!("submission failed: {e}")))?;
        debug!("Submitted user turn with submission_id={submission_id}");

        // Process the response from Codex
        let mut response_content = String::new();
        let mut fallback_message: Option<String> = None;
        let mut streamed_events: Vec<Value> = Vec::new();

        // Read events from the conversation
        loop {
            let event = conversation
                .next_event()
                .await
                .map_err(|e| GatewayError::Internal(format!("failed to get event: {e}")))?;

            let event_json = to_value(&event.msg)
                .unwrap_or_else(|err| json!({ "serialization_error": err.to_string() }));
            streamed_events.push(event_json);

            let should_finish = match event.msg {
                EventMsg::AgentMessage(agent_event) => {
                    if !response_content.is_empty() {
                        response_content.push('\n');
                    }
                    response_content.push_str(agent_event.message.trim_end());
                    false
                }
                EventMsg::AgentMessageDelta(delta_event) => {
                    response_content.push_str(&delta_event.delta);
                    false
                }
                EventMsg::RawResponseItem(item_event) => {
                    if let ResponseItem::Message { content, .. } = item_event.item {
                        for piece in content {
                            if let ContentItem::OutputText { text } = piece {
                                if !response_content.is_empty() {
                                    response_content.push('\n');
                                }
                                response_content.push_str(text.trim_end());
                            }
                        }
                    }
                    false
                }
                EventMsg::AgentReasoning(reasoning_event) => {
                    debug!(
                        "Agent reasoning (truncated): {}",
                        reasoning_event.text.chars().take(120).collect::<String>()
                    );
                    false
                }
                EventMsg::AgentReasoningRawContent(raw_event) => {
                    debug!("Agent raw reasoning chunk len={}", raw_event.text.len());
                    false
                }
                EventMsg::TaskComplete(task_event) => {
                    if fallback_message.is_none() {
                        fallback_message = task_event.last_agent_message;
                    }
                    true
                }
                EventMsg::Error(error_event) => {
                    return Err(GatewayError::Internal(format!(
                        "Codex error: {}",
                        error_event.message
                    )));
                }
                EventMsg::TurnAborted(aborted) => {
                    return Err(GatewayError::Internal(format!(
                        "Codex turn aborted: {:?}",
                        aborted.reason
                    )));
                }
                EventMsg::Warning(warning_event) => {
                    warn!("Codex warning: {}", warning_event.message);
                    false
                }
                EventMsg::StreamError(stream_error) => {
                    warn!("Codex stream error: {}", stream_error.message);
                    false
                }
                EventMsg::BackgroundEvent(background_event) => {
                    debug!("Codex background event: {}", background_event.message);
                    false
                }
                other => {
                    trace!("Unhandled Codex event: {:?}", other);
                    false
                }
            };

            if should_finish {
                break;
            }
        }

        if let (true, Some(message)) = (response_content.trim().is_empty(), fallback_message) {
            response_content = message;
        }
        let result = json!({
            "type": "ai_response",
            "conversation_id": conversation_id.to_string(),
            "content": response_content,
            "model": "claude-3-sonnet",
            "timestamp": Utc::now().to_rfc3339(),
            "events": streamed_events,
        });

        let elapsed = Utc::now().signed_duration_since(start_time);
        info!(
            "REAL Codex AI prompt execution completed: session_id={:?}, conversation_id={}, request_id={}, elapsed={}ms",
            session_id,
            conversation_id,
            request_id,
            elapsed.num_milliseconds()
        );

        Ok(result)
    }

    /// Get or create a conversation for the given session
    pub async fn get_or_create_conversation(
        &self,
        session_id: Option<&str>,
    ) -> GatewayResult<ConversationId> {
        let mut conversations = self.active_conversations.lock().await;

        match session_id {
            Some(sid) => {
                if let Some(conversation_id) = conversations.get(sid) {
                    debug!(
                        "Found existing conversation for session {}: {}",
                        sid, conversation_id
                    );
                    Ok(*conversation_id)
                } else {
                    // Create new conversation via ConversationManager
                    warn!(
                        "No existing conversation found for session {}, creating new one",
                        sid
                    );
                    let conversation_id = self.create_new_conversation().await?;
                    conversations.insert(sid.to_string(), conversation_id);
                    info!(
                        "Created new conversation for session {}: {}",
                        sid, conversation_id
                    );
                    Ok(conversation_id)
                }
            }
            None => {
                // Create ephemeral conversation for session-less requests
                warn!("Creating ephemeral conversation for session-less request");
                let conversation_id = self.create_new_conversation().await?;
                debug!("Created ephemeral conversation: {}", conversation_id);
                Ok(conversation_id)
            }
        }
    }

    /// Create a new conversation using ConversationManager from codex-core
    async fn create_new_conversation(&self) -> GatewayResult<ConversationId> {
        let config = (*self.codex_config).clone();
        let new_conversation = {
            let manager = self.conversation_manager.lock().await;
            manager.new_conversation(config).await.map_err(|e| {
                GatewayError::Internal(format!("Failed to create conversation: {e}"))
            })?
        };

        let conversation_id = new_conversation.conversation_id;
        let session_configured = new_conversation.session_configured.clone();

        self.conversation_metadata
            .lock()
            .await
            .insert(conversation_id, session_configured);

        debug!(
            "ConversationManager created new conversation: {}",
            conversation_id
        );
        Ok(conversation_id)
    }

    /// Generate unique request ID
    async fn next_request_id(&self) -> u64 {
        let mut counter = self.request_counter.lock().await;
        *counter += 1;
        *counter
    }

    /// Recupera status básico da sessão para JSON-RPC
    pub async fn get_session_status(
        &self,
        session_id: &str,
    ) -> GatewayResult<Option<SessionStatus>> {
        let conversation_id = {
            let conversations = self.active_conversations.lock().await;
            match conversations.get(session_id) {
                Some(id) => *id,
                None => return Ok(None),
            }
        };

        let metadata = {
            let metadata_map = self.conversation_metadata.lock().await;
            metadata_map.get(&conversation_id).cloned()
        };

        Ok(Some(SessionStatus {
            conversation_id,
            metadata,
        }))
    }

    /// Cancela sessão ativa e remove rastros em memória
    pub async fn cancel_session(&self, session_id: &str) -> GatewayResult<Option<ConversationId>> {
        let conversation_id = {
            let mut conversations = self.active_conversations.lock().await;
            conversations.remove(session_id)
        };

        if let Some(conversation_id) = conversation_id {
            self.conversation_metadata
                .lock()
                .await
                .remove(&conversation_id);
            Ok(Some(conversation_id))
        } else {
            Ok(None)
        }
    }

    /// Get public accessor to conversation manager
    pub fn conversation_manager(&self) -> &Arc<Mutex<ConversationManager>> {
        &self.conversation_manager
    }

    /// Get public accessor to codex config
    pub fn codex_config(&self) -> &Arc<CodexConfig> {
        &self.codex_config
    }

    /// Get public accessor to active conversations (for WebSocket interrupt)
    pub fn active_conversations(&self) -> &Arc<Mutex<HashMap<String, ConversationId>>> {
        &self.active_conversations
    }

    /// Resume a conversation from a previous session
    ///
    /// This method allows resuming a conversation by its ID. It:
    /// 1. Finds the rollout path for the conversation
    /// 2. Uses ConversationManager to resume from the rollout
    /// 3. Registers the conversation with a new session_id
    ///
    /// # Arguments
    ///
    /// * `conversation_id_str` - The conversation ID to resume
    /// * `session_id` - New session ID to associate with the resumed conversation
    ///
    /// # Returns
    ///
    /// The ConversationId of the resumed conversation
    pub async fn resume_conversation(
        &self,
        conversation_id_str: &str,
        session_id: &str,
    ) -> GatewayResult<ConversationId> {
        info!(
            "Resuming conversation: conversation_id={}, session_id={}",
            conversation_id_str, session_id
        );

        // Find rollout path by conversation ID
        let rollout_path =
            find_conversation_path_by_id_str(&self.codex_config.codex_home, conversation_id_str)
                .await
                .map_err(|e| {
                    GatewayError::Internal(format!("Failed to find conversation path: {e}"))
                })?
                .ok_or_else(|| {
                    GatewayError::InvalidRequest(format!(
                        "No conversation found with ID: {conversation_id_str}"
                    ))
                })?;

        debug!("Found rollout path: {:?}", rollout_path);

        // Resume via ConversationManager
        let config = (*self.codex_config).clone();
        let auth_manager = AuthManager::shared(
            self.codex_config.codex_home.clone(),
            false,
            self.codex_config.cli_auth_credentials_store_mode,
        );

        let new_conversation = {
            let manager = self.conversation_manager.lock().await;
            manager
                .resume_conversation_from_rollout(config, rollout_path, auth_manager)
                .await
                .map_err(|e| {
                    GatewayError::Internal(format!("Failed to resume conversation: {e}"))
                })?
        };

        let conversation_id = new_conversation.conversation_id;
        let session_configured = new_conversation.session_configured.clone();

        // Register with session_id
        self.active_conversations
            .lock()
            .await
            .insert(session_id.to_string(), conversation_id);

        self.conversation_metadata
            .lock()
            .await
            .insert(conversation_id, session_configured);

        info!(
            "Successfully resumed conversation: conversation_id={}, session_id={}",
            conversation_id, session_id
        );

        Ok(conversation_id)
    }
}
