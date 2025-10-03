# ⚡ Cloud Functions - Event-Driven Wrapper

## Visão Geral

Wrapper event-driven para codex-rs usando Cloud Run (simulando Cloud Functions) com Google Cloud Platform. Ideal para processamento assíncrono, cargas esparsas e integração com eventos de storage.

## Arquitetura Event-Driven

```
Cloud Storage → Pub/Sub → Cloud Run (Rust Functions) → codex-app-server
      │                      ↓                             ↓
   Triggers            Event Processing                 Results
      │                      ↓                             ↓
HTTP Requests → API Gateway → Function Router → Cloud Storage (outputs)
```

## Serviços GCP

- **Cloud Run** (simulando Functions)
- **Cloud Pub/Sub** - Event messaging
- **Cloud Storage** - File triggers + outputs
- **Cloud Scheduler** - Cron triggers
- **Cloud Firestore** - Job queue + status
- **Cloud Logging** - Centralized logs
- **Cloud Workflows** - Complex orchestration

## Stack Rust

### Dependências (Cargo.toml)

```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
google-cloud-pubsub = "0.25"
google-cloud-storage = "0.24"
google-cloud-firestore = "0.54"
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = "0.3"
base64 = "0.22"
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.11", features = ["json"] }
clap = { version = "4", features = ["derive"] }
```

## Implementação

### HTTP Function - Enqueue Job (enqueue/src/main.rs)

```rust
use axum::{extract::Json, response::Json as ResponseJson, routing::post, Router};
use google_cloud_pubsub::{client::Client as PubSubClient, publisher::Publisher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
struct EnqueueRequest {
    prompt: String,
    timeout_ms: Option<u64>,
    callback_url: Option<String>,
    priority: Option<String>,
    metadata: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
struct EnqueueResponse {
    job_id: String,
    status: String,
    message: String,
    estimated_completion: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct CodexJob {
    id: String,
    prompt: String,
    timeout_ms: Option<u64>,
    callback_url: Option<String>,
    priority: String,
    metadata: HashMap<String, String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn enqueue_job_handler(
    Json(req): Json<EnqueueRequest>,
) -> Result<ResponseJson<EnqueueResponse>, axum::http::StatusCode> {
    let job_id = uuid::Uuid::new_v4().to_string();
    
    let job = CodexJob {
        id: job_id.clone(),
        prompt: req.prompt,
        timeout_ms: req.timeout_ms,
        callback_url: req.callback_url,
        priority: req.priority.unwrap_or_else(|| "normal".to_string()),
        metadata: req.metadata.unwrap_or_default(),
        created_at: chrono::Utc::now(),
    };
    
    // Publish job to Pub/Sub
    match publish_job_to_queue(&job).await {
        Ok(_) => {
            let estimated_completion = chrono::Utc::now() + chrono::Duration::seconds(
                (job.timeout_ms.unwrap_or(30000) / 1000) as i64 + 30
            );
            
            Ok(ResponseJson(EnqueueResponse {
                job_id,
                status: "queued".to_string(),
                message: "Job queued for processing".to_string(),
                estimated_completion: Some(estimated_completion.to_rfc3339()),
            }))
        },
        Err(e) => {
            tracing::error!("Failed to enqueue job: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn publish_job_to_queue(job: &CodexJob) -> Result<(), Box<dyn std::error::Error>> {
    let client = PubSubClient::default().await?;
    let topic = client.topic("codex-jobs");
    let publisher = topic.new_publisher(None);
    
    let message = serde_json::to_string(job)?;
    let mut message_builder = publisher.new_message(message.as_bytes());
    
    // Add attributes for routing
    message_builder = message_builder
        .add_attribute("priority", &job.priority)
        .add_attribute("job_id", &job.id);
    
    publisher.publish(message_builder).await?;
    
    Ok(())
}

async fn get_job_status_handler(
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<ResponseJson<serde_json::Value>, axum::http::StatusCode> {
    // Query Firestore for job status
    match get_job_status(&job_id).await {
        Ok(status) => Ok(ResponseJson(status)),
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

async fn get_job_status(job_id: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use google_cloud_firestore::*;
    
    let db = FirestoreDb::new("your-project-id").await?;
    
    let result: Option<serde_json::Value> = db
        .fluent()
        .select()
        .from("jobs")
        .obj()
        .document_id(job_id)
        .query()
        .await?;
    
    result.ok_or_else(|| "Job not found".into())
}

async fn health_handler() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    
    let app = Router::new()
        .route("/enqueue", post(enqueue_job_handler))
        .route("/status/:job_id", axum::routing::get(get_job_status_handler))
        .route("/health", axum::routing::get(health_handler));
    
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    
    tracing::info!("Enqueue service running on port {}", port);
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

### Pub/Sub Function - Process Job (processor/src/main.rs)

```rust
use axum::{extract::Json, routing::post, Router};
use google_cloud_storage::client::Client as StorageClient;
use google_cloud_firestore::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Deserialize)]
struct PubSubMessage {
    #[serde(rename = "data")]
    data: String,
    attributes: HashMap<String, String>,
    #[serde(rename = "messageId")]
    message_id: String,
    #[serde(rename = "publishTime")]
    publish_time: String,
}

#[derive(Deserialize)]
struct PubSubEnvelope {
    message: PubSubMessage,
    subscription: String,
}

#[derive(Deserialize, Serialize)]
struct CodexJob {
    id: String,
    prompt: String,
    timeout_ms: Option<u64>,
    callback_url: Option<String>,
    priority: String,
    metadata: HashMap<String, String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct JobResult {
    job_id: String,
    status: String,
    exit_code: i32,
    stdout: String,
    stderr: String,
    execution_time_ms: u64,
    output_files: Vec<String>,
    created_files: Vec<OutputFile>,
    completed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct OutputFile {
    filename: String,
    size_bytes: u64,
    content_type: String,
    storage_path: String,
}

async fn process_job_handler(
    Json(envelope): Json<PubSubEnvelope>,
) -> Result<&'static str, axum::http::StatusCode> {
    tracing::info!("Processing Pub/Sub message: {}", envelope.message.message_id);
    
    // Decode base64 message
    let job_data = base64::decode(&envelope.message.data)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    
    let job: CodexJob = serde_json::from_slice(&job_data)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    
    tracing::info!("Processing job: {} with priority: {}", job.id, job.priority);
    
    // Update job status to "processing"
    if let Err(e) = update_job_status(&job.id, "processing", None).await {
        tracing::error!("Failed to update job status: {}", e);
    }
    
    // Execute job
    match execute_codex_job(&job).await {
        Ok(result) => {
            // Save result to storage
            if let Err(e) = save_job_result(&result).await {
                tracing::error!("Failed to save job result: {}", e);
            }
            
            // Update job status to "completed"
            if let Err(e) = update_job_status(&job.id, "completed", Some(&result)).await {
                tracing::error!("Failed to update job status: {}", e);
            }
            
            // Send callback if specified
            if let Some(callback_url) = &job.callback_url {
                if let Err(e) = send_callback(callback_url, &result).await {
                    tracing::error!("Failed to send callback: {}", e);
                }
            }
            
            tracing::info!("Job {} completed successfully", job.id);
        },
        Err(e) => {
            tracing::error!("Job {} failed: {}", job.id, e);
            
            let error_result = JobResult {
                job_id: job.id.clone(),
                status: "failed".to_string(),
                exit_code: -1,
                stdout: String::new(),
                stderr: e.to_string(),
                execution_time_ms: 0,
                output_files: Vec::new(),
                created_files: Vec::new(),
                completed_at: chrono::Utc::now(),
            };
            
            if let Err(e) = update_job_status(&job.id, "failed", Some(&error_result)).await {
                tracing::error!("Failed to update failed job status: {}", e);
            }
        }
    }
    
    Ok("Job processed")
}

async fn execute_codex_job(job: &CodexJob) -> Result<JobResult, Box<dyn std::error::Error>> {
    let start_time = std::time::Instant::now();
    
    // Create temporary workspace
    let workspace_dir = format!("/tmp/codex-{}", job.id);
    tokio::fs::create_dir_all(&workspace_dir).await?;
    
    // Spawn codex-app-server
    let mut child = tokio::process::Command::new("./codex-app-server")
        .current_dir(&workspace_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let mut stdin = child.stdin.take().unwrap();
    
    // Send initialization
    let init_cmd = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {
                "name": "cloud-functions",
                "version": "1.0.0"
            }
        }
    });
    
    // Send execution command
    let exec_cmd = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "execOneOffCommand",
        "params": {
            "command": [job.prompt.clone()],
            "timeoutMs": job.timeout_ms.unwrap_or(30000),
            "cwd": workspace_dir
        }
    });
    
    // Write to stdin
    stdin.write_all(format!("{}\n{}\n", init_cmd, exec_cmd).as_bytes()).await?;
    stdin.flush().await?;
    drop(stdin);
    
    // Wait for completion with timeout
    let timeout_duration = std::time::Duration::from_millis(
        job.timeout_ms.unwrap_or(30000) + 10000
    );
    
    let output = tokio::time::timeout(timeout_duration, child.wait_with_output())
        .await
        .map_err(|_| "Execution timeout")?
        .map_err(|e| format!("Process error: {}", e))?;
    
    let execution_time = start_time.elapsed().as_millis() as u64;
    
    // Parse JSON-RPC responses
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    
    // Scan for created files
    let created_files = scan_created_files(&workspace_dir).await?;
    
    // Upload files to storage
    let uploaded_files = upload_files_to_storage(&job.id, &created_files).await?;
    
    // Cleanup workspace
    if let Err(e) = tokio::fs::remove_dir_all(&workspace_dir).await {
        tracing::warn!("Failed to cleanup workspace: {}", e);
    }
    
    Ok(JobResult {
        job_id: job.id.clone(),
        status: "completed".to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout: stdout_str.to_string(),
        stderr: stderr_str.to_string(),
        execution_time_ms: execution_time,
        output_files: uploaded_files.iter().map(|f| f.storage_path.clone()).collect(),
        created_files: uploaded_files,
        completed_at: chrono::Utc::now(),
    })
}

async fn scan_created_files(workspace_dir: &str) -> Result<Vec<(String, std::path::PathBuf)>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    let mut dir = tokio::fs::read_dir(workspace_dir).await?;
    
    while let Some(entry) = dir.next_entry().await? {
        if entry.file_type().await?.is_file() {
            let filename = entry.file_name().to_string_lossy().to_string();
            files.push((filename, entry.path()));
        }
    }
    
    Ok(files)
}

async fn upload_files_to_storage(
    job_id: &str,
    files: &[(String, std::path::PathBuf)],
) -> Result<Vec<OutputFile>, Box<dyn std::error::Error>> {
    let storage_client = StorageClient::default();
    let bucket = "codex-job-outputs";
    let mut uploaded_files = Vec::new();
    
    for (filename, path) in files {
        let file_content = tokio::fs::read(path).await?;
        let metadata = tokio::fs::metadata(path).await?;
        
        let object_name = format!("jobs/{}/outputs/{}", job_id, filename);
        
        storage_client
            .upload_object(bucket, &object_name, file_content.clone())
            .await?;
        
        let content_type = mime_guess::from_path(filename)
            .first_or_octet_stream()
            .to_string();
        
        uploaded_files.push(OutputFile {
            filename: filename.clone(),
            size_bytes: metadata.len(),
            content_type,
            storage_path: format!("gs://{}/{}", bucket, object_name),
        });
    }
    
    Ok(uploaded_files)
}

async fn update_job_status(
    job_id: &str,
    status: &str,
    result: Option<&JobResult>,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = FirestoreDb::new("your-project-id").await?;
    
    let mut update_data = serde_json::json!({
        "status": status,
        "updated_at": chrono::Utc::now().to_rfc3339()
    });
    
    if let Some(result) = result {
        update_data["result"] = serde_json::to_value(result)?;
    }
    
    db.fluent()
        .update()
        .in_col("jobs")
        .document_id(job_id)
        .object(&update_data)
        .execute()
        .await?;
    
    Ok(())
}

async fn save_job_result(result: &JobResult) -> Result<(), Box<dyn std::error::Error>> {
    let storage_client = StorageClient::default();
    let bucket = "codex-job-results";
    let object_name = format!("jobs/{}/result.json", result.job_id);
    
    let result_json = serde_json::to_string_pretty(result)?;
    
    storage_client
        .upload_object(bucket, &object_name, result_json.into_bytes())
        .await?;
    
    Ok(())
}

async fn send_callback(
    callback_url: &str,
    result: &JobResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let response = client
        .post(callback_url)
        .json(result)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("Callback failed with status: {}", response.status()).into());
    }
    
    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    
    let app = Router::new()
        .route("/process", post(process_job_handler))
        .route("/health", axum::routing::get(health_handler));
    
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    
    tracing::info!("Job processor running on port {}", port);
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

### Storage Function - File Upload Processor (storage/src/main.rs)

```rust
use axum::{extract::Json, routing::post, Router};
use google_cloud_storage::client::Client as StorageClient;
use google_cloud_pubsub::{client::Client as PubSubClient, publisher::Publisher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
struct StorageEvent {
    #[serde(rename = "bucketId")]
    bucket_id: String,
    #[serde(rename = "objectId")]
    object_id: String,
    #[serde(rename = "eventType")]
    event_type: String,
    #[serde(rename = "timeCreated")]
    time_created: String,
}

#[derive(Serialize)]
struct AutoProcessJob {
    id: String,
    prompt: String,
    timeout_ms: u64,
    storage_path: String,
    auto_generated: bool,
    metadata: HashMap<String, String>,
}

async fn process_file_upload_handler(
    Json(event): Json<StorageEvent>,
) -> Result<&'static str, axum::http::StatusCode> {
    tracing::info!("Processing storage event: {} for object: {}", event.event_type, event.object_id);
    
    match event.event_type.as_str() {
        "OBJECT_FINALIZE" => {
            if event.object_id.ends_with(".codex") || event.object_id.ends_with(".prompt") {
                // Auto-process uploaded prompt files
                if let Err(e) = process_prompt_file(&event).await {
                    tracing::error!("Failed to process prompt file: {}", e);
                    return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                }
            } else if event.object_id.ends_with(".zip") || event.object_id.ends_with(".tar.gz") {
                // Auto-extract and process archives
                if let Err(e) = process_archive_file(&event).await {
                    tracing::error!("Failed to process archive file: {}", e);
                    return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        },
        "OBJECT_DELETE" => {
            // Cleanup related resources
            if let Err(e) = cleanup_object_resources(&event).await {
                tracing::error!("Failed to cleanup resources: {}", e);
            }
        },
        _ => {
            tracing::info!("Ignoring event type: {}", event.event_type);
        }
    }
    
    Ok("Event processed")
}

async fn process_prompt_file(event: &StorageEvent) -> Result<(), Box<dyn std::error::Error>> {
    let storage_client = StorageClient::default();
    
    // Download file content
    let file_content = storage_client
        .download_object(&event.bucket_id, &event.object_id)
        .await?;
    
    let prompt = String::from_utf8_lossy(&file_content);
    
    // Extract metadata from file path
    let path_parts: Vec<&str> = event.object_id.split('/').collect();
    let filename = path_parts.last().unwrap_or("unknown");
    
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "file_upload".to_string());
    metadata.insert("original_filename".to_string(), filename.to_string());
    metadata.insert("bucket".to_string(), event.bucket_id.clone());
    
    // Determine timeout based on file size
    let timeout_ms = match file_content.len() {
        0..=1000 => 30000,      // 30s for small prompts
        1001..=10000 => 120000, // 2m for medium prompts  
        _ => 300000,            // 5m for large prompts
    };
    
    // Create and queue job
    let job = AutoProcessJob {
        id: uuid::Uuid::new_v4().to_string(),
        prompt: prompt.to_string(),
        timeout_ms,
        storage_path: format!("gs://{}/{}", event.bucket_id, event.object_id),
        auto_generated: true,
        metadata,
    };
    
    // Queue for processing
    let pubsub_client = PubSubClient::default().await?;
    let topic = pubsub_client.topic("codex-jobs");
    let publisher = topic.new_publisher(None);
    
    let message = serde_json::to_string(&job)?;
    let mut message_builder = publisher.new_message(message.as_bytes());
    
    message_builder = message_builder
        .add_attribute("priority", "low")
        .add_attribute("source", "file_upload")
        .add_attribute("job_id", &job.id);
    
    publisher.publish(message_builder).await?;
    
    tracing::info!("Queued auto-processing job {} for file {}", job.id, event.object_id);
    
    Ok(())
}

async fn process_archive_file(event: &StorageEvent) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Processing archive file: {}", event.object_id);
    
    // Download and extract archive
    // Create multiple jobs for extracted files
    // This is a simplified implementation
    
    Ok(())
}

async fn cleanup_object_resources(event: &StorageEvent) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Cleaning up resources for deleted object: {}", event.object_id);
    
    // Cancel any pending jobs related to this file
    // Cleanup temporary resources
    // This is a simplified implementation
    
    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    
    let app = Router::new()
        .route("/storage-event", post(process_file_upload_handler))
        .route("/health", axum::routing::get(health_handler));
    
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    
    tracing::info!("Storage processor running on port {}", port);
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

## Configuração e Deploy

### 1. Configurar Projeto GCP

```bash
# Variáveis
export PROJECT_ID=your-project-id
export REGION=us-central1

# Habilitar APIs
gcloud services enable \
  run.googleapis.com \
  pubsub.googleapis.com \
  storage.googleapis.com \
  firestore.googleapis.com \
  cloudbuild.googleapis.com
```

### 2. Criar Recursos

```bash
# Criar tópicos Pub/Sub
gcloud pubsub topics create codex-jobs
gcloud pubsub topics create codex-results

# Criar subscriptions
gcloud pubsub subscriptions create codex-jobs-processor \
  --topic=codex-jobs \
  --ack-deadline=600

# Criar buckets
gsutil mb -p $PROJECT_ID -c STANDARD -l $REGION gs://codex-uploads-$PROJECT_ID
gsutil mb -p $PROJECT_ID -c STANDARD -l $REGION gs://codex-job-outputs-$PROJECT_ID
gsutil mb -p $PROJECT_ID -c STANDARD -l $REGION gs://codex-job-results-$PROJECT_ID

# Configurar Firestore
gcloud firestore databases create --region=$REGION
```

### 3. Build e Deploy Functions

```bash
# Build enqueue function
cd enqueue
gcloud builds submit --tag gcr.io/$PROJECT_ID/codex-enqueue

# Deploy enqueue function
gcloud run deploy codex-enqueue \
  --image gcr.io/$PROJECT_ID/codex-enqueue \
  --platform managed \
  --region $REGION \
  --allow-unauthenticated \
  --memory 1Gi \
  --timeout 60s \
  --max-instances 100

# Build processor function
cd ../processor
gcloud builds submit --tag gcr.io/$PROJECT_ID/codex-processor

# Deploy processor function
gcloud run deploy codex-processor \
  --image gcr.io/$PROJECT_ID/codex-processor \
  --platform managed \
  --region $REGION \
  --memory 2Gi \
  --timeout 540s \
  --max-instances 50

# Build storage function
cd ../storage
gcloud builds submit --tag gcr.io/$PROJECT_ID/codex-storage

# Deploy storage function
gcloud run deploy codex-storage \
  --image gcr.io/$PROJECT_ID/codex-storage \
  --platform managed \
  --region $REGION \
  --allow-unauthenticated \
  --memory 1Gi \
  --timeout 60s
```

### 4. Configurar Pub/Sub Push Subscription

```bash
# Configurar push endpoint para processor
PROCESSOR_URL=$(gcloud run services describe codex-processor --region=$REGION --format="value(status.url)")

gcloud pubsub subscriptions modify-push-config codex-jobs-processor \
  --push-endpoint="$PROCESSOR_URL/process"
```

### 5. Configurar Storage Triggers

```bash
# Configurar trigger para uploads
STORAGE_URL=$(gcloud run services describe codex-storage --region=$REGION --format="value(status.url)")

# Criar notification para bucket
gsutil notification create -t codex-storage-events -f json gs://codex-uploads-$PROJECT_ID

# Configurar Cloud Function trigger (alternativa)
gcloud functions deploy codex-file-processor \
  --gen2 \
  --runtime=nodejs20 \
  --source=./cloud-function-trigger \
  --entry-point=handleStorageEvent \
  --trigger-bucket=codex-uploads-$PROJECT_ID \
  --set-env-vars=STORAGE_PROCESSOR_URL=$STORAGE_URL
```

## Uso das APIs

### 1. Enfileirar Job

```bash
curl -X POST https://codex-enqueue-hash.a.run.app/enqueue \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "create a python web server with FastAPI",
    "timeout_ms": 120000,
    "callback_url": "https://my-app.com/webhook",
    "priority": "high",
    "metadata": {
      "user_id": "user123",
      "project": "web-app"
    }
  }'
```

### Resposta

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "queued",
  "message": "Job queued for processing",
  "estimated_completion": "2024-01-01T12:05:00Z"
}
```

### 2. Verificar Status do Job

```bash
curl https://codex-enqueue-hash.a.run.app/status/550e8400-e29b-41d4-a716-446655440000
```

### Resposta

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "result": {
    "exit_code": 0,
    "stdout": "FastAPI server created successfully",
    "execution_time_ms": 45000,
    "created_files": [
      {
        "filename": "main.py",
        "size_bytes": 1024,
        "content_type": "text/x-python",
        "storage_path": "gs://codex-job-outputs/jobs/550e8400/outputs/main.py"
      }
    ]
  }
}
```

### 3. Upload de Arquivo para Processamento Automático

```bash
# Upload arquivo .codex para processamento automático
gsutil cp my-prompt.codex gs://codex-uploads-$PROJECT_ID/prompts/

# O arquivo será automaticamente processado
```

### 4. Webhook Callback

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "exit_code": 0,
  "stdout": "Application created successfully",
  "stderr": "",
  "execution_time_ms": 45000,
  "output_files": ["gs://codex-job-outputs/jobs/550e8400/outputs/main.py"],
  "completed_at": "2024-01-01T12:04:30Z"
}
```

## Monitoramento

### Cloud Logging

```bash
# Ver logs de todas as functions
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name:(codex-enqueue OR codex-processor OR codex-storage)" --limit 100

# Ver logs de um job específico
gcloud logging read "jsonPayload.job_id=\"550e8400-e29b-41d4-a716-446655440000\"" --limit 50
```

### Pub/Sub Monitoring

```bash
# Ver métricas de subscription
gcloud pubsub subscriptions describe codex-jobs-processor

# Ver mensagens não processadas
gcloud pubsub subscriptions pull codex-jobs-processor --auto-ack --limit=5
```

### Storage Events

```bash
# Ver events de storage
gcloud logging read "resource.type=gcs_bucket AND resource.labels.bucket_name=codex-uploads-$PROJECT_ID" --limit 20
```

## Custos Estimados

| Componente | Uso | Custo Mensal |
|------------|-----|--------------|
| Cloud Run | 10k requests | $10 |
| Pub/Sub | 1M messages | $5 |
| Cloud Storage | 100GB | $5 |
| Firestore | 1M operations | $3 |
| Cloud Logging | Standard | $2 |
| **Total** |  | **$25** |

## Vantagens

- ✅ **Custo baixíssimo** - Pay-per-use extremo
- ✅ **Event-driven** - Processamento automático
- ✅ **Escalabilidade massiva** - 0→1000 automático
- ✅ **Integração nativa** - GCP services
- ✅ **Resiliente** - Dead letter queues
- ✅ **Observabilidade** - Logs centralizados

## Limitações

- ❌ **Cold start** - 1-3s primeira execução
- ❌ **Timeout limitado** - Max 60min por function
- ❌ **Complexidade** - Múltiplos serviços
- ❌ **Debug complexo** - Fluxo distribuído
- ❌ **Latência variável** - Dependente de eventos

## Casos de Uso Ideais

1. **Processamento de lote** - Arquivos grandes
2. **Integrações webhook** - APIs de terceiros
3. **Pipelines CI/CD** - Deploy automático
4. **Data processing** - ETL workflows
5. **Background jobs** - Tarefas não críticas

## Troubleshooting

### Debug de Job Falso

```bash
# Ver logs de job específico
gcloud logging read "jsonPayload.job_id=\"job-id-here\"" --format="table(timestamp,jsonPayload.message)"

# Ver status no Firestore
gcloud firestore documents get projects/$PROJECT_ID/databases/(default)/documents/jobs/job-id-here
```

### Pub/Sub Issues

```bash
# Ver mensagens deadletter
gcloud pubsub subscriptions pull codex-jobs-processor-deadletter --auto-ack --limit=10

# Reprocessar mensagem
gcloud pubsub topics publish codex-jobs --message="job-data-here"
```

### Storage Triggers

```bash
# Verificar notifications
gsutil notification list gs://codex-uploads-$PROJECT_ID

# Testar manualmente
curl -X POST $STORAGE_URL/storage-event \
  -H "Content-Type: application/json" \
  -d '{"bucketId":"test","objectId":"test.codex","eventType":"OBJECT_FINALIZE"}'
```

## Conclusão

Cloud Functions (via Cloud Run) é ideal para workloads event-driven e processamento assíncrono do codex-rs. Oferece custo ultra-baixo e escalabilidade massiva, perfeito para integrações e pipelines automatizados.

**Próximo passo:** Combinar com outros padrões em arquitetura híbrida para casos de uso mais complexos.