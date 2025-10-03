# 🏗️ Hybrid Architecture - Cloud-Native Microservices

## Visão Geral

Arquitetura híbrida que combina o melhor de cada serviço GCP para criar um wrapper completo e escalável para codex-rs. Ideal para empresas que precisam de máxima flexibilidade, performance e observabilidade.

## Arquitetura Completa

```
                            Internet
                               │
                    ┌──────────▼──────────┐
                    │  Cloud Load Balancer │
                    │   + Cloud Armor      │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │   API Gateway        │
                    │   (Cloud Endpoints)  │
                    └─────┬────────────┬───┘
                          │            │
              ┌───────────▼───┐    ┌───▼──────────┐
              │  REST API      │    │  WebSocket   │
              │  (Cloud Run)   │    │  (GKE)       │
              └───────┬───────┘    └───┬──────────┘
                      │                │
           ┌──────────▼──────────┐     │
           │    Cloud Pub/Sub     │     │
           │   (Job Queue)        │     │
           └─────┬────────────────┘     │
                 │                      │
     ┌───────────▼──────────┐          │
     │  Processing Workers   │          │
     │  (GKE StatefulSet)    │          │
     └───────┬──────────────┘          │
             │                         │
     ┌───────▼─────────┐              │
     │ Event Functions │              │
     │ (Cloud Run)     │              │
     └─────────────────┘              │
             │                        │
    ┌────────▼─────────────────────────▼──┐
    │         Storage Layer                │
    │  ┌─────────┬──────────┬──────────┐  │
    │  │Cloud    │Cloud     │Memory    │  │
    │  │Storage  │Firestore │Store     │  │
    │  │(Files)  │(Meta)    │(Cache)   │  │
    │  └─────────┴──────────┴──────────┘  │
    └─────────────────────────────────────┘
```

## Componentes da Arquitetura

### 1. API Gateway (Cloud Endpoints)

```yaml
# api-gateway.yaml
swagger: "2.0"
info:
  title: "Codex Cloud API"
  version: "2.0.0"
  description: "Unified API for Codex-RS Cloud Services"
host: "api.codex-cloud.com"
schemes:
  - "https"
basePath: "/v2"

x-google-management:
  metrics:
  - name: "requests_total"
    displayName: "Total Requests"
    valueType: INT64
    metricKind: CUMULATIVE
  quota:
    limits:
    - name: "requests_per_minute"
      metric: "requests_total"
      unit: "1/min"
      values:
        STANDARD: 1000
        PREMIUM: 10000

security:
  - api_key: []
  - oauth2: ["read", "write"]

paths:
  /exec:
    post:
      summary: "Execute command synchronously"
      operationId: "executeSync"
      x-google-backend:
        address: "https://codex-api-run-hash.a.run.app"
        path_translation: APPEND_PATH_TO_ADDRESS
      parameters:
      - name: "body"
        in: "body"
        required: true
        schema:
          $ref: "#/definitions/ExecRequest"
      responses:
        200:
          description: "Execution result"
          schema:
            $ref: "#/definitions/ExecResponse"

  /exec/async:
    post:
      summary: "Execute command asynchronously"
      operationId: "executeAsync"
      x-google-backend:
        address: "https://codex-jobs-run-hash.a.run.app"
      responses:
        202:
          description: "Job queued"

  /stream:
    get:
      summary: "WebSocket streaming endpoint"
      operationId: "streamExecution"
      x-google-backend:
        address: "https://codex-ws-gke-hash.example.com"

  /jobs/{jobId}:
    get:
      summary: "Get job status"
      operationId: "getJobStatus"
      x-google-backend:
        address: "https://codex-status-run-hash.a.run.app"

securityDefinitions:
  api_key:
    type: "apiKey"
    name: "X-API-KEY"
    in: "header"
  oauth2:
    type: "oauth2"
    authorizationUrl: "https://accounts.google.com/o/oauth2/auth"
    flow: "implicit"
    scopes:
      read: "Read access"
      write: "Write access"
```

### 2. REST API Service (Cloud Run)

```rust
// api-service/src/main.rs
use axum::{
    extract::{Path, Query, State},
    middleware::{self, Next},
    response::{Json, Response},
    routing::{get, post},
    Router,
};
use google_cloud_pubsub::{client::Client as PubSubClient, publisher::Publisher};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error, instrument};

#[derive(Clone)]
struct AppState {
    pubsub_client: PubSubClient,
    redis_client: redis::Client,
    config: Arc<ServiceConfig>,
}

#[derive(Debug)]
struct ServiceConfig {
    worker_pool_url: String,
    job_timeout_default: u64,
    rate_limit_per_minute: u32,
}

#[derive(Deserialize)]
struct ExecRequest {
    prompt: String,
    mode: Option<ExecutionMode>,
    timeout_ms: Option<u64>,
    priority: Option<Priority>,
    metadata: Option<HashMap<String, String>>,
    session_id: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ExecutionMode {
    Sync,     // Immediate response via worker pool
    Async,    // Queue job via Pub/Sub
    Stream,   // WebSocket redirect
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Serialize)]
struct ExecResponse {
    session_id: String,
    mode: ExecutionMode,
    #[serde(flatten)]
    result: ExecutionResult,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ExecutionResult {
    Immediate {
        exit_code: i32,
        stdout: String,
        stderr: String,
        execution_time_ms: u64,
    },
    Queued {
        job_id: String,
        estimated_completion: String,
        status_url: String,
    },
    Streaming {
        websocket_url: String,
        connection_token: String,
    },
}

// Rate limiting middleware
async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, axum::http::StatusCode> {
    let api_key = req.headers()
        .get("X-API-KEY")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous");
    
    let mut redis_conn = state.redis_client.get_async_connection().await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let key = format!("rate_limit:{}", api_key);
    let current: i32 = redis_conn.incr(&key, 1).await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if current == 1 {
        let _: () = redis_conn.expire(&key, 60).await
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    
    if current > state.config.rate_limit_per_minute as i32 {
        return Err(axum::http::StatusCode::TOO_MANY_REQUESTS);
    }
    
    Ok(next.run(req).await)
}

#[instrument(skip(state))]
async fn exec_handler(
    State(state): State<AppState>,
    Json(req): Json<ExecRequest>,
) -> Result<Json<ExecResponse>, axum::http::StatusCode> {
    let session_id = req.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mode = req.mode.unwrap_or(ExecutionMode::Sync);
    
    info!(
        session_id = %session_id,
        mode = ?mode,
        prompt_length = req.prompt.len(),
        "Processing execution request"
    );
    
    let result = match mode {
        ExecutionMode::Sync => {
            execute_immediate(&state, &req, &session_id).await?
        },
        ExecutionMode::Async => {
            execute_async(&state, &req, &session_id).await?
        },
        ExecutionMode::Stream => {
            execute_streaming(&state, &req, &session_id).await?
        },
    };
    
    Ok(Json(ExecResponse {
        session_id,
        mode,
        result,
    }))
}

async fn execute_immediate(
    state: &AppState,
    req: &ExecRequest,
    session_id: &str,
) -> Result<ExecutionResult, axum::http::StatusCode> {
    let client = reqwest::Client::new();
    
    let worker_request = serde_json::json!({
        "prompt": req.prompt,
        "timeout_ms": req.timeout_ms.unwrap_or(state.config.job_timeout_default),
        "session_id": session_id,
        "metadata": req.metadata.as_ref().unwrap_or(&HashMap::new())
    });
    
    let start_time = std::time::Instant::now();
    
    let response = client
        .post(&format!("{}/execute", state.config.worker_pool_url))
        .json(&worker_request)
        .timeout(std::time::Duration::from_millis(
            req.timeout_ms.unwrap_or(state.config.job_timeout_default) + 5000
        ))
        .send()
        .await
        .map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
    
    if response.status().is_success() {
        let worker_response: serde_json::Value = response.json().await
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ExecutionResult::Immediate {
            exit_code: worker_response["exit_code"].as_i64().unwrap_or(-1) as i32,
            stdout: worker_response["stdout"].as_str().unwrap_or("").to_string(),
            stderr: worker_response["stderr"].as_str().unwrap_or("").to_string(),
            execution_time_ms: execution_time,
        })
    } else {
        error!("Worker request failed with status: {}", response.status());
        Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn execute_async(
    state: &AppState,
    req: &ExecRequest,
    session_id: &str,
) -> Result<ExecutionResult, axum::http::StatusCode> {
    let job_id = uuid::Uuid::new_v4().to_string();
    
    let job = serde_json::json!({
        "id": job_id,
        "session_id": session_id,
        "prompt": req.prompt,
        "timeout_ms": req.timeout_ms.unwrap_or(state.config.job_timeout_default),
        "priority": req.priority.as_ref().unwrap_or(&Priority::Normal),
        "metadata": req.metadata.as_ref().unwrap_or(&HashMap::new()),
        "created_at": chrono::Utc::now().to_rfc3339()
    });
    
    // Publish to Pub/Sub
    let topic = state.pubsub_client.topic("codex-jobs");
    let publisher = topic.new_publisher(None);
    
    let message = serde_json::to_string(&job)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut message_builder = publisher.new_message(message.as_bytes());
    message_builder = message_builder
        .add_attribute("job_id", &job_id)
        .add_attribute("priority", &format!("{:?}", req.priority.as_ref().unwrap_or(&Priority::Normal)));
    
    publisher.publish(message_builder).await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let estimated_completion = chrono::Utc::now() + chrono::Duration::seconds(
        (req.timeout_ms.unwrap_or(state.config.job_timeout_default) / 1000) as i64 + 60
    );
    
    Ok(ExecutionResult::Queued {
        job_id: job_id.clone(),
        estimated_completion: estimated_completion.to_rfc3339(),
        status_url: format!("/v2/jobs/{}", job_id),
    })
}

async fn execute_streaming(
    _state: &AppState,
    _req: &ExecRequest,
    session_id: &str,
) -> Result<ExecutionResult, axum::http::StatusCode> {
    // Generate WebSocket connection token
    let connection_token = uuid::Uuid::new_v4().to_string();
    
    // In production, store token in Redis with TTL
    
    Ok(ExecutionResult::Streaming {
        websocket_url: format!("wss://ws.codex-cloud.com/stream/{}", session_id),
        connection_token,
    })
}

async fn get_job_status_handler(
    Path(job_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    // Query job status from Firestore
    // This is a simplified implementation
    let status = serde_json::json!({
        "job_id": job_id,
        "status": "processing",
        "progress": 50,
        "estimated_completion": chrono::Utc::now().to_rfc3339()
    });
    
    Ok(Json(status))
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn metrics_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    // Collect metrics from Redis and other sources
    let metrics = serde_json::json!({
        "requests_per_minute": 42,
        "active_sessions": 15,
        "queue_depth": 8,
        "average_response_time_ms": 1250
    });
    
    Json(metrics)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    
    let pubsub_client = PubSubClient::default().await?;
    let redis_client = redis::Client::open(
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
    )?;
    
    let config = Arc::new(ServiceConfig {
        worker_pool_url: std::env::var("WORKER_POOL_URL")
            .unwrap_or_else(|_| "http://codex-workers:8080".to_string()),
        job_timeout_default: 30000,
        rate_limit_per_minute: 100,
    });
    
    let state = AppState {
        pubsub_client,
        redis_client,
        config,
    };
    
    let app = Router::new()
        .route("/exec", post(exec_handler))
        .route("/jobs/:job_id", get(get_job_status_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .with_state(state);
    
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    
    info!("API service running on port {}", port);
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

### 3. WebSocket Service (GKE)

```rust
// ws-service/src/main.rs
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn, error, instrument};

#[derive(Clone)]
struct WsState {
    connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    worker_pool: Arc<WorkerPool>,
    redis_client: redis::Client,
    broadcast_tx: broadcast::Sender<BroadcastMessage>,
}

#[derive(Clone)]
struct ConnectionInfo {
    session_id: String,
    user_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
struct WorkerPool {
    workers: Vec<WorkerNode>,
    current_index: Arc<std::sync::atomic::AtomicUsize>,
}

#[derive(Clone)]
struct WorkerNode {
    id: String,
    endpoint: String,
    health: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Serialize, Deserialize)]
struct StreamingRequest {
    prompt: String,
    session_id: Option<String>,
    stream_type: Option<StreamType>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StreamType {
    Interactive,  // Real-time interaction
    Progress,     // Progress updates only
    Full,         // Everything (default)
}

#[derive(Serialize, Deserialize, Clone)]
struct StreamingResponse {
    session_id: String,
    message_type: MessageType,
    data: serde_json::Value,
    timestamp: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum MessageType {
    Progress,
    Output,
    Error,
    Complete,
    Heartbeat,
}

#[derive(Serialize, Deserialize, Clone)]
struct BroadcastMessage {
    target: BroadcastTarget,
    message: StreamingResponse,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum BroadcastTarget {
    All,
    Session(String),
    User(String),
}

impl WorkerPool {
    async fn get_worker_for_session(&self, session_id: &str) -> Option<&WorkerNode> {
        // Use consistent hashing to assign sessions to workers
        let hash = md5::compute(session_id.as_bytes());
        let index = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) as usize % self.workers.len();
        
        let worker = &self.workers[index];
        if worker.health.load(std::sync::atomic::Ordering::Relaxed) {
            Some(worker)
        } else {
            // Fallback to round-robin
            self.get_available_worker()
        }
    }
    
    fn get_available_worker(&self) -> Option<&WorkerNode> {
        let start = self.current_index.load(std::sync::atomic::Ordering::Relaxed);
        
        for i in 0..self.workers.len() {
            let idx = (start + i) % self.workers.len();
            let worker = &self.workers[idx];
            
            if worker.health.load(std::sync::atomic::Ordering::Relaxed) {
                self.current_index.store((idx + 1) % self.workers.len(), std::sync::atomic::Ordering::Relaxed);
                return Some(worker);
            }
        }
        
        None
    }
}

#[instrument(skip(ws, state))]
async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(state): State<WsState>,
) -> Response {
    info!(session_id = %session_id, "WebSocket connection requested");
    ws.on_upgrade(move |socket| handle_websocket(socket, session_id, state))
}

async fn handle_websocket(socket: WebSocket, session_id: String, state: WsState) {
    let (mut sender, mut receiver) = socket.split();
    
    // Register connection
    let connection_info = ConnectionInfo {
        session_id: session_id.clone(),
        user_id: None, // Extract from auth token in production
        created_at: chrono::Utc::now(),
        last_activity: chrono::Utc::now(),
    };
    
    state.connections.write().await.insert(session_id.clone(), connection_info);
    
    // Subscribe to broadcasts
    let mut broadcast_rx = state.broadcast_tx.subscribe();
    
    // Spawn task to handle broadcasts
    let session_id_broadcast = session_id.clone();
    let mut sender_broadcast = sender.clone();
    tokio::spawn(async move {
        while let Ok(broadcast) = broadcast_rx.recv().await {
            let should_send = match &broadcast.target {
                BroadcastTarget::All => true,
                BroadcastTarget::Session(id) => id == &session_id_broadcast,
                BroadcastTarget::User(_) => false, // Implement user targeting
            };
            
            if should_send {
                if let Ok(json) = serde_json::to_string(&broadcast.message) {
                    if sender_broadcast.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
    
    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<StreamingRequest>(&text) {
                        let state_clone = state.clone();
                        let session_id_clone = session_id.clone();
                        
                        tokio::spawn(async move {
                            handle_streaming_request(request, session_id_clone, state_clone).await;
                        });
                    }
                },
                Message::Ping(ping) => {
                    if sender.send(Message::Pong(ping)).await.is_err() {
                        break;
                    }
                },
                Message::Close(_) => {
                    break;
                },
                _ => {}
            }
        } else {
            break;
        }
    }
    
    // Cleanup connection
    state.connections.write().await.remove(&session_id);
    info!(session_id = %session_id, "WebSocket connection closed");
}

async fn handle_streaming_request(
    request: StreamingRequest,
    session_id: String,
    state: WsState,
) {
    let actual_session_id = request.session_id.unwrap_or(session_id);
    
    // Get worker for this session
    let worker = match state.worker_pool.get_worker_for_session(&actual_session_id).await {
        Some(worker) => worker,
        None => {
            send_error(&state, &actual_session_id, "No workers available").await;
            return;
        }
    };
    
    // Send progress update
    send_progress(&state, &actual_session_id, "Starting execution", 0).await;
    
    // Execute on worker with streaming
    match execute_streaming_on_worker(worker, &request, &actual_session_id).await {
        Ok(_) => {
            send_complete(&state, &actual_session_id, "Execution completed successfully").await;
        },
        Err(e) => {
            send_error(&state, &actual_session_id, &format!("Execution failed: {}", e)).await;
        }
    }
}

async fn execute_streaming_on_worker(
    worker: &WorkerNode,
    request: &StreamingRequest,
    session_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to worker WebSocket
    let url = format!("ws://{}/stream", worker.endpoint.replace("http://", "").replace("https://", ""));
    
    // For simplicity, using HTTP requests here
    // In production, establish WebSocket connection to worker
    let client = reqwest::Client::new();
    
    let worker_request = serde_json::json!({
        "prompt": request.prompt,
        "session_id": session_id,
        "stream": true
    });
    
    let response = client
        .post(&format!("{}/execute", worker.endpoint))
        .json(&worker_request)
        .send()
        .await?;
    
    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        
        // Send output (simplified)
        // In production, stream real-time output
        
        Ok(())
    } else {
        Err(format!("Worker request failed: {}", response.status()).into())
    }
}

async fn send_progress(state: &WsState, session_id: &str, message: &str, percentage: u8) {
    let response = StreamingResponse {
        session_id: session_id.to_string(),
        message_type: MessageType::Progress,
        data: serde_json::json!({
            "message": message,
            "percentage": percentage
        }),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    let broadcast = BroadcastMessage {
        target: BroadcastTarget::Session(session_id.to_string()),
        message: response,
    };
    
    let _ = state.broadcast_tx.send(broadcast);
}

async fn send_output(state: &WsState, session_id: &str, stdout: &str, stderr: &str) {
    let response = StreamingResponse {
        session_id: session_id.to_string(),
        message_type: MessageType::Output,
        data: serde_json::json!({
            "stdout": stdout,
            "stderr": stderr
        }),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    let broadcast = BroadcastMessage {
        target: BroadcastTarget::Session(session_id.to_string()),
        message: response,
    };
    
    let _ = state.broadcast_tx.send(broadcast);
}

async fn send_complete(state: &WsState, session_id: &str, message: &str) {
    let response = StreamingResponse {
        session_id: session_id.to_string(),
        message_type: MessageType::Complete,
        data: serde_json::json!({
            "message": message
        }),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    let broadcast = BroadcastMessage {
        target: BroadcastTarget::Session(session_id.to_string()),
        message: response,
    };
    
    let _ = state.broadcast_tx.send(broadcast);
}

async fn send_error(state: &WsState, session_id: &str, error: &str) {
    let response = StreamingResponse {
        session_id: session_id.to_string(),
        message_type: MessageType::Error,
        data: serde_json::json!({
            "error": error
        }),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    let broadcast = BroadcastMessage {
        target: BroadcastTarget::Session(session_id.to_string()),
        message: response,
    };
    
    let _ = state.broadcast_tx.send(broadcast);
}

async fn get_connections_handler(
    State(state): State<WsState>,
) -> axum::Json<HashMap<String, ConnectionInfo>> {
    let connections = state.connections.read().await.clone();
    axum::Json(connections)
}

async fn broadcast_handler(
    State(state): State<WsState>,
    axum::Json(message): axum::Json<BroadcastMessage>,
) -> &'static str {
    let _ = state.broadcast_tx.send(message);
    "Broadcast sent"
}

async fn health_handler() -> &'static str {
    "OK"
}

// Health check for workers
async fn health_check_workers(worker_pool: Arc<WorkerPool>) {
    let client = reqwest::Client::new();
    
    loop {
        for worker in &worker_pool.workers {
            let health_result = client
                .get(&format!("{}/health", worker.endpoint))
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await;
            
            let is_healthy = health_result
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            
            worker.health.store(is_healthy, std::sync::atomic::Ordering::Relaxed);
            
            if !is_healthy {
                warn!("Worker {} is unhealthy", worker.id);
            }
        }
        
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    
    let redis_client = redis::Client::open(
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379".to_string())
    )?;
    
    // Initialize worker pool
    let worker_endpoints: Vec<String> = std::env::var("WORKER_ENDPOINTS")
        .unwrap_or_else(|_| "http://worker-0:7000,http://worker-1:7000".to_string())
        .split(',')
        .map(|s| s.to_string())
        .collect();
    
    let workers = worker_endpoints.into_iter().enumerate()
        .map(|(i, endpoint)| WorkerNode {
            id: format!("worker-{}", i),
            endpoint,
            health: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        })
        .collect();
    
    let worker_pool = Arc::new(WorkerPool {
        workers,
        current_index: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    });
    
    // Start health checking
    tokio::spawn(health_check_workers(worker_pool.clone()));
    
    let (broadcast_tx, _) = broadcast::channel(1000);
    
    let state = WsState {
        connections: Arc::new(RwLock::new(HashMap::new())),
        worker_pool,
        redis_client,
        broadcast_tx,
    };
    
    let app = Router::new()
        .route("/stream/:session_id", get(websocket_handler))
        .route("/connections", get(get_connections_handler))
        .route("/broadcast", axum::routing::post(broadcast_handler))
        .route("/health", get(health_handler))
        .with_state(state);
    
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    
    info!("WebSocket service running on port {}", port);
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

### 4. Infrastructure as Code (Terraform)

```hcl
# terraform/main.tf
terraform {
  required_version = ">= 1.0"
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }
}

variable "project_id" {
  description = "GCP Project ID"
  type        = string
}

variable "region" {
  description = "GCP Region"
  type        = string
  default     = "us-central1"
}

variable "environment" {
  description = "Environment (dev/staging/prod)"
  type        = string
  default     = "dev"
}

locals {
  resource_prefix = "codex-${var.environment}"
}

# Enable APIs
resource "google_project_service" "apis" {
  for_each = toset([
    "container.googleapis.com",
    "run.googleapis.com",
    "pubsub.googleapis.com",
    "storage.googleapis.com",
    "firestore.googleapis.com",
    "redis.googleapis.com",
    "monitoring.googleapis.com",
    "logging.googleapis.com",
    "servicecontrol.googleapis.com",
    "endpoints.googleapis.com"
  ])
  
  project = var.project_id
  service = each.value
  
  disable_dependent_services = true
}

# GKE Cluster
module "gke_cluster" {
  source = "./modules/gke"
  
  project_id = var.project_id
  region     = var.region
  
  cluster_name = "${local.resource_prefix}-cluster"
  
  node_pools = {
    "system" = {
      machine_type = "e2-medium"
      min_nodes    = 1
      max_nodes    = 3
      disk_size    = 50
      preemptible  = false
    }
    "workers" = {
      machine_type = "e2-standard-4"
      min_nodes    = 2
      max_nodes    = 10
      disk_size    = 100
      preemptible  = var.environment != "prod"
    }
    "websocket" = {
      machine_type = "e2-standard-2" 
      min_nodes    = 1
      max_nodes    = 5
      disk_size    = 50
      preemptible  = var.environment != "prod"
    }
  }
  
  depends_on = [google_project_service.apis]
}

# Cloud Run Services
module "cloud_run_services" {
  source = "./modules/cloud-run"
  
  project_id = var.project_id
  region     = var.region
  
  services = {
    "${local.resource_prefix}-api" = {
      image       = "gcr.io/${var.project_id}/codex-api:latest"
      memory      = "1Gi"
      cpu         = "1000m"
      concurrency = 80
      env_vars = {
        REDIS_URL = module.redis.connection_string
        WORKER_POOL_URL = "http://${module.gke_cluster.workers_service_ip}:8080"
      }
    }
    "${local.resource_prefix}-processor" = {
      image       = "gcr.io/${var.project_id}/codex-processor:latest"
      memory      = "2Gi" 
      cpu         = "2000m"
      timeout     = 540
      concurrency = 10
      env_vars = {
        PROJECT_ID = var.project_id
      }
    }
    "${local.resource_prefix}-storage" = {
      image       = "gcr.io/${var.project_id}/codex-storage:latest"
      memory      = "512Mi"
      cpu         = "500m"
      concurrency = 100
      env_vars = {
        PROJECT_ID = var.project_id
      }
    }
  }
  
  depends_on = [google_project_service.apis]
}

# Redis Instance
module "redis" {
  source = "./modules/redis"
  
  project_id = var.project_id
  region     = var.region
  
  instance_name = "${local.resource_prefix}-cache"
  memory_size   = var.environment == "prod" ? 5 : 1
  tier          = var.environment == "prod" ? "STANDARD_HA" : "BASIC"
  
  depends_on = [google_project_service.apis]
}

# Pub/Sub Topics and Subscriptions
module "pubsub" {
  source = "./modules/pubsub"
  
  project_id = var.project_id
  
  topics = {
    "${local.resource_prefix}-jobs" = {
      message_retention = "7d"
    }
    "${local.resource_prefix}-results" = {
      message_retention = "3d"
    }
    "${local.resource_prefix}-events" = {
      message_retention = "1d"
    }
  }
  
  subscriptions = {
    "${local.resource_prefix}-jobs-processor" = {
      topic        = "${local.resource_prefix}-jobs"
      push_endpoint = module.cloud_run_services.service_urls["${local.resource_prefix}-processor"]
      ack_deadline = "600s"
    }
  }
  
  depends_on = [google_project_service.apis, module.cloud_run_services]
}

# Storage Buckets
module "storage" {
  source = "./modules/storage"
  
  project_id = var.project_id
  region     = var.region
  
  buckets = {
    "${local.resource_prefix}-uploads" = {
      location = var.region
      versioning = false
      lifecycle_rules = [
        {
          action = "Delete"
          condition = {
            age = 30
          }
        }
      ]
    }
    "${local.resource_prefix}-outputs" = {
      location = var.region
      versioning = true
      lifecycle_rules = [
        {
          action = "Delete"
          condition = {
            age = 90
          }
        }
      ]
    }
    "${local.resource_prefix}-results" = {
      location = var.region
      versioning = false
      lifecycle_rules = [
        {
          action = "Delete"
          condition = {
            age = 365
          }
        }
      ]
    }
  }
  
  # Configure storage triggers
  notification_configs = {
    "${local.resource_prefix}-uploads" = {
      topic = "${local.resource_prefix}-storage-events"
      events = ["OBJECT_FINALIZE", "OBJECT_DELETE"]
    }
  }
  
  depends_on = [google_project_service.apis]
}

# Firestore Database
resource "google_firestore_database" "database" {
  project     = var.project_id
  name        = "(default)"
  location_id = var.region
  type        = "FIRESTORE_NATIVE"
  
  depends_on = [google_project_service.apis]
}

# API Gateway
module "api_gateway" {
  source = "./modules/api-gateway"
  
  project_id = var.project_id
  region     = var.region
  
  api_config = {
    name = "${local.resource_prefix}-api"
    spec_file = "api-gateway.yaml"
  }
  
  backend_services = {
    api_service = module.cloud_run_services.service_urls["${local.resource_prefix}-api"]
    ws_service  = "https://${module.gke_cluster.websocket_ingress_ip}"
  }
  
  depends_on = [module.cloud_run_services, module.gke_cluster]
}

# Monitoring and Alerting
module "monitoring" {
  source = "./modules/monitoring"
  
  project_id = var.project_id
  
  alert_policies = {
    "high_error_rate" = {
      display_name = "High Error Rate"
      conditions = [
        {
          filter = "resource.type=\"cloud_run_revision\""
          comparison = "COMPARISON_GREATER_THAN"
          threshold_value = 0.1
        }
      ]
    }
    "high_latency" = {
      display_name = "High Latency"
      conditions = [
        {
          filter = "resource.type=\"cloud_run_revision\""
          comparison = "COMPARISON_GREATER_THAN"
          threshold_value = 5000
        }
      ]
    }
  }
  
  depends_on = [google_project_service.apis]
}

# Outputs
output "api_gateway_url" {
  value = module.api_gateway.gateway_url
}

output "websocket_url" {
  value = "wss://${module.gke_cluster.websocket_ingress_ip}/stream"
}

output "redis_connection" {
  value = module.redis.connection_string
  sensitive = true
}

output "storage_buckets" {
  value = module.storage.bucket_urls
}
```

## Deploy e Configuração

### 1. Preparação do Ambiente

```bash
# Variáveis
export PROJECT_ID=your-project-id
export REGION=us-central1
export ENVIRONMENT=dev

# Autenticação
gcloud auth login
gcloud config set project $PROJECT_ID

# Terraform
cd terraform
terraform init
terraform plan -var="project_id=$PROJECT_ID" -var="region=$REGION" -var="environment=$ENVIRONMENT"
terraform apply
```

### 2. Build e Deploy de Imagens

```bash
# Build todas as imagens
services=("api-service" "ws-service" "processor" "storage")

for service in "${services[@]}"; do
  echo "Building $service..."
  cd $service
  gcloud builds submit --tag gcr.io/$PROJECT_ID/codex-$service:latest
  cd ..
done
```

### 3. Deploy no Kubernetes

```bash
# Configurar kubectl
gcloud container clusters get-credentials codex-dev-cluster --region=$REGION

# Apply manifests
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmaps/
kubectl apply -f k8s/secrets/
kubectl apply -f k8s/deployments/
kubectl apply -f k8s/services/
kubectl apply -f k8s/ingress/
```

### 4. Configurar API Gateway

```bash
# Deploy API config
gcloud endpoints services deploy api-gateway.yaml

# Configurar autenticação
gcloud endpoints configs create --service=api.codex-cloud.com
```

## Uso da API Híbrida

### 1. Execução Síncrona (Cloud Run)

```bash
curl -X POST https://api.codex-cloud.com/v2/exec \
  -H "X-API-KEY: your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "create a web application with authentication",
    "mode": "sync",
    "timeout_ms": 60000
  }'
```

### 2. Execução Assíncrona (Pub/Sub + Workers)

```bash
curl -X POST https://api.codex-cloud.com/v2/exec \
  -H "X-API-KEY: your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "build a complete e-commerce platform",
    "mode": "async",
    "priority": "high",
    "timeout_ms": 300000
  }'
```

### 3. Streaming WebSocket (GKE)

```javascript
const ws = new WebSocket('wss://api.codex-cloud.com/v2/stream/my-session-id', [], {
  headers: {
    'X-API-KEY': 'your-api-key'
  }
});

ws.onopen = () => {
  ws.send(JSON.stringify({
    prompt: "create an interactive game",
    stream_type: "interactive"
  }));
};

ws.onmessage = (event) => {
  const response = JSON.parse(event.data);
  console.log(`[${response.message_type}]`, response.data);
};
```

## Monitoramento e Observabilidade

### Dashboards Customizados

```yaml
# monitoring/dashboard.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: codex-dashboard
data:
  dashboard.json: |
    {
      "dashboard": {
        "title": "Codex Cloud Platform",
        "panels": [
          {
            "title": "Request Rate",
            "type": "graph",
            "targets": [
              {
                "expr": "rate(http_requests_total[5m])",
                "legendFormat": "{{service}}"
              }
            ]
          },
          {
            "title": "Response Time",
            "type": "graph", 
            "targets": [
              {
                "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))",
                "legendFormat": "95th percentile"
              }
            ]
          },
          {
            "title": "Error Rate",
            "type": "singlestat",
            "targets": [
              {
                "expr": "rate(http_requests_total{status=~\"5..\"}[5m]) / rate(http_requests_total[5m])",
                "legendFormat": "Error Rate"
              }
            ]
          }
        ]
      }
    }
```

### Alerting Rules

```yaml
# monitoring/alerts.yaml
groups:
- name: codex-alerts
  rules:
  - alert: HighErrorRate
    expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High error rate detected"
      description: "Error rate is {{ $value }} for {{ $labels.service }}"
      
  - alert: HighLatency
    expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 5
    for: 2m
    labels:
      severity: warning
    annotations:
      summary: "High latency detected"
      description: "95th percentile latency is {{ $value }}s"
      
  - alert: WorkerUnhealthy
    expr: up{job="codex-workers"} == 0
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "Codex worker is down"
      description: "Worker {{ $labels.instance }} is not responding"
```

## Custos Estimados

| Componente | Recursos | Custo Mensal (Dev) | Custo Mensal (Prod) |
|------------|----------|-------------------|---------------------|
| GKE Cluster | 3-8 nodes | $200-400 | $800-1500 |
| Cloud Run | 10k-100k requests | $20-100 | $200-500 |
| Redis MemoryStore | 1-5GB | $30-150 | $300-750 |
| Pub/Sub | 1M-10M messages | $5-50 | $50-200 |
| Cloud Storage | 100GB-1TB | $10-50 | $100-300 |
| Load Balancer | Standard | $20 | $50 |
| Monitoring/Logging | Standard | $30 | $150 |
| **Total** |  | **$315-820** | **$1650-3450** |

## Vantagens da Arquitetura Híbrida

- ✅ **Flexibilidade máxima** - Múltiplos padrões de execução
- ✅ **Performance otimizada** - Serviço certo para cada workload
- ✅ **Escalabilidade diferenciada** - Auto-scaling por componente
- ✅ **Observabilidade completa** - Métricas e logs unificados
- ✅ **Resiliência** - Failover entre serviços
- ✅ **Custo otimizado** - Pay-per-use onde possível

## Limitações

- ❌ **Complexidade alta** - Múltiplos serviços para gerenciar
- ❌ **Curva de aprendizado** - Requer expertise em múltiplas tecnologias
- ❌ **Overhead operacional** - Deploy e monitoring complexos
- ❌ **Debug distribuído** - Troubleshooting em múltiplos serviços
- ❌ **Custo fixo base** - GKE cluster sempre rodando

## Quando Usar Arquitetura Híbrida

### ✅ Ideal para:

1. **Empresas grandes** - Múltiplos teams e casos de uso
2. **Produção crítica** - SLAs rigorosos e alta disponibilidade
3. **Crescimento rápido** - Necessidade de escalar diferentes componentes
4. **Integrações complexas** - APIs, webhooks, eventos
5. **Compliance** - Controle total sobre dados e processamento

### ❌ Evitar quando:

1. **Time pequeno** - Falta de expertise DevOps
2. **Orçamento limitado** - Custos fixos altos
3. **MVP/Protótipo** - Over-engineering desnecessário
4. **Caso de uso simples** - Cloud Run seria suficiente

## Roadmap de Migração

### Fase 1: Fundação (Mês 1-2)
- [ ] Deploy GKE cluster
- [ ] Configurar Redis e storage
- [ ] Implementar API básica

### Fase 2: Core Services (Mês 2-3)
- [ ] Workers no GKE
- [ ] Pub/Sub processing
- [ ] WebSocket streaming

### Fase 3: Integration (Mês 3-4)
- [ ] API Gateway
- [ ] Event-driven functions
- [ ] Monitoring completo

### Fase 4: Optimization (Mês 4-6)
- [ ] Auto-scaling tuning
- [ ] Performance optimization
- [ ] Cost optimization

## Conclusão

A arquitetura híbrida oferece a solução mais completa e flexível para wrapper cloud do codex-rs. Combina performance, escalabilidade e observabilidade, ideal para casos de uso empresariais complexos que justifiquem o investimento em infraestrutura e expertise.

**Recomendação:** Comece com Cloud Run, evolua para GKE conforme necessário, e implemente componentes híbridos gradualmente baseado em necessidades reais.