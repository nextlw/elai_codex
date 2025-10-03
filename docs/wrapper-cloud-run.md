# 🏃 Cloud Run - Wrapper Serverless Simples

## Visão Geral

Wrapper serverless para codex-rs usando Google Cloud Run. Ideal para MVPs, startups e cargas baixas-médias com foco em simplicidade e baixo custo.

## Arquitetura

```
Internet → Cloud Load Balancer → Cloud Run (Rust) → codex-app-server
                                      ↓
                                Cloud Storage (sessions/files)
                                      ↓  
                                Cloud Firestore (metadata)
```

## Serviços GCP Utilizados

- **Cloud Run** - Container serverless (escala 0→1000)
- **Cloud Storage** - Arquivos de sessão e outputs
- **Cloud Firestore** - Metadados e session tracking
- **Cloud Load Balancer** - Distribuição de carga
- **Cloud Logging** - Logs centralizados
- **Cloud Monitoring** - Métricas e alertas

## Stack Rust

### Dependências (Cargo.toml)

```toml
[dependencies]
axum = { version = "0.8", features = ["sse"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
futures = "0.3"
google-cloud-storage = "0.24"
google-cloud-firestore = "0.54"
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }
```

## Implementação

### main.rs

```rust
use axum::{
    extract::State, 
    http::StatusCode, 
    response::{Json, Sse, sse::Event}, 
    routing::{get, post}, 
    Router
};
use futures::stream::{Stream, StreamExt};
use google_cloud_storage::client::Client as StorageClient;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, process::{Command, Stdio}};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

#[derive(Deserialize)]
struct ExecRequest {
    prompt: String,
    timeout_ms: Option<u64>,
    session_id: Option<String>,
}

#[derive(Serialize)]
struct ExecResponse {
    session_id: String,
    exit_code: i32,
    stdout: String,
    stderr: String,
    execution_time_ms: u64,
}

#[derive(Clone)]
struct AppState {
    storage_client: StorageClient,
    bucket_name: String,
}

// 🔥 NOVO: Endpoint SSE para streaming em tempo real
async fn exec_stream_handler(
    State(state): State<AppState>,
    Json(req): Json<ExecRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::unbounded_channel();
    let session_id = req.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    tokio::spawn(async move {
        let start_time = std::time::Instant::now();
        
        // Enviar evento inicial
        let _ = tx.send(Event::default()
            .event("task_started")
            .data(serde_json::json!({
                "session_id": session_id,
                "status": "initializing"
            }).to_string()));
        
        // Spawn codex-app-server process
        let mut child = match Command::new("./codex-app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn() {
            Ok(child) => child,
            Err(e) => {
                let _ = tx.send(Event::default()
                    .event("error")
                    .data(format!("Failed to spawn process: {}", e)));
                return;
            }
        };

        let mut stdin = child.stdin.take().unwrap();
        
        // Initialize session
        let init_cmd = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "cloud-wrapper",
                    "version": "1.0.0"
                }
            }
        });
        
        // Execute command
        let exec_cmd = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "execOneOffCommand",
            "params": {
                "command": [req.prompt],
                "timeoutMs": req.timeout_ms.unwrap_or(30000)
            }
        });
        
        // Send commands
        if let Err(e) = stdin.write_all(format!("{}\n{}\n", init_cmd, exec_cmd).as_bytes()).await {
            let _ = tx.send(Event::default()
                .event("error")
                .data(format!("Failed to write to stdin: {}", e)));
            return;
        }
        let _ = stdin.flush().await;
        
        // 🔥 Stream responses JSON-RPC linha por linha
        let mut stdout_reader = BufReader::new(child.stdout.take().unwrap());
        let mut stdout_lines = stdout_reader.lines();
        
        let mut final_result = None;
        let mut task_files_created = Vec::new();
        let mut task_progress = Vec::new();
        
        while let Ok(Some(line)) = stdout_lines.next_line().await {
            // Enviar linha bruta como evento de progresso
            let _ = tx.send(Event::default()
                .event("stdout_line")
                .data(line.clone()));
            
            // Parse JSON-RPC se possível
            if let Ok(json_msg) = serde_json::from_str::<serde_json::Value>(&line) {
                match json_msg.get("method").and_then(|m| m.as_str()) {
                    Some("task/progress") => {
                        let _ = tx.send(Event::default()
                            .event("task_progress")
                            .data(json_msg.to_string()));
                    },
                    _ => {
                        // Response ou notification genérica
                        if json_msg.get("id") == Some(&serde_json::Value::Number(2.into())) {
                            // execOneOffCommand response
                            if let Some(result) = json_msg.get("result") {
                                final_result = Some(result.clone());
                                
                                let _ = tx.send(Event::default()
                                    .event("task_result")
                                    .data(result.to_string()));
                            }
                        }
                    }
                }
            }
        }
        
        // Aguardar finalização do processo
        let exit_status = child.wait().await.unwrap_or_else(|_| std::process::ExitStatus::from_raw(1));
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        // Evento final
        let final_response = serde_json::json!({
            "session_id": session_id,
            "exit_code": exit_status.code().unwrap_or(-1),
            "execution_time_ms": execution_time,
            "final_result": final_result,
            "files_created": task_files_created,
            "status": if exit_status.success() { "completed" } else { "failed" }
        });
        
        // Salvar no Cloud Storage
        let _ = save_session_to_storage(&state, &session_id, &final_response).await;
        
        // Enviar resposta final
        let _ = tx.send(Event::default()
            .event("task_completed")
            .data(final_response.to_string()));
    });
    
    Sse::new(ReceiverStream::new(rx))
}

// 📞 LEGACY: Endpoint tradicional (para compatibilidade)
async fn exec_handler(
    State(state): State<AppState>,
    Json(req): Json<ExecRequest>,
) -> Result<Json<ExecResponse>, StatusCode> {
    // Implementação simplificada que delega para o SSE internamente
    // e retorna apenas o resultado final
    let session_id = req.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    // TODO: Implementar versão que espera conclusão e retorna resultado final
    // Por enquanto, retorna erro sugerindo uso do endpoint SSE
    Err(StatusCode::UNPROCESSABLE_ENTITY)
}

async fn save_session_to_storage(
    state: &AppState,
    session_id: &str,
    data: &serde_json::Value,
) -> Result<(), StatusCode> {
    let object_name = format!("sessions/{}/output.json", session_id);
    
    state.storage_client
        .upload_object(&state.bucket_name, &object_name, data.to_string().into_bytes())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    
    let storage_client = StorageClient::default();
    let bucket_name = std::env::var("BUCKET_NAME")
        .unwrap_or_else(|_| "codex-sessions".to_string());
    
    let state = AppState {
        storage_client,
        bucket_name,
    };
    
    let app = Router::new()
        .route("/api/v1/exec", post(exec_handler))              // Legacy endpoint
        .route("/api/v1/exec/stream", post(exec_stream_handler)) // 🔥 SSE endpoint
        .route("/health", get(health_handler))
        .with_state(state);
    
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    
    tracing::info!("Server running on port {}", port);
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

## Dockerfile

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/codex-wrapper /usr/local/bin/
COPY --from=builder /app/target/debug/codex-app-server /usr/local/bin/

EXPOSE 8080
CMD ["codex-wrapper"]
```

## Deploy

### Configuração Inicial

```bash
# Configurar projeto
export PROJECT_ID=your-project-id
export REGION=us-central1

# Habilitar APIs
gcloud services enable \
  run.googleapis.com \
  storage.googleapis.com \
  firestore.googleapis.com \
  cloudbuild.googleapis.com
```

### Build e Deploy

```bash
# Build e push da imagem
gcloud builds submit --tag gcr.io/$PROJECT_ID/codex-wrapper

# Deploy no Cloud Run
gcloud run deploy codex-wrapper \
  --image gcr.io/$PROJECT_ID/codex-wrapper \
  --platform managed \
  --region $REGION \
  --allow-unauthenticated \
  --memory 2Gi \
  --cpu 2 \
  --max-instances 100 \
  --set-env-vars BUCKET_NAME=codex-sessions-$PROJECT_ID
```

### Criar Storage Bucket

```bash
# Criar bucket para sessões
gsutil mb -p $PROJECT_ID -c STANDARD -l $REGION gs://codex-sessions-$PROJECT_ID

# Configurar CORS
cat > cors.json << EOF
[
  {
    "origin": ["*"],
    "method": ["GET", "POST"],
    "responseHeader": ["Content-Type"],
    "maxAgeSeconds": 3600
  }
]
EOF

gsutil cors set cors.json gs://codex-sessions-$PROJECT_ID
```

## Uso da API

### 🔥 Executar Comando com Streaming (Recomendado)

```bash
# Server-Sent Events - mantém conexão até conclusão
curl -N -X POST https://codex-wrapper-hash.a.run.app/api/v1/exec/stream \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "prompt": "create a python script that prints hello world",
    "timeout_ms": 30000
  }'
```

### Resposta Streaming (SSE)

```bash
# Eventos em tempo real
event: task_started
data: {"session_id":"abc123","status":"initializing"}

event: stdout_line  
data: {"jsonrpc":"2.0","id":1,"result":{"initialized":true}}

event: task_progress
data: {"method":"task/progress","params":{"step":"analyzing_prompt","progress":25}}

event: stdout_line
data: {"jsonrpc":"2.0","id":2,"result":{"files_created":["hello.py"],"summary":"Created Python script"}}

event: task_result
data: {"files_created":["hello.py"],"summary":"Created Python script successfully"}

event: task_completed
data: {"session_id":"abc123","exit_code":0,"execution_time_ms":1250,"status":"completed"}
```

### 📞 Executar Comando Tradicional (Legacy)

```bash
curl -X POST https://codex-wrapper-hash.a.run.app/api/v1/exec \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "create a python script that prints hello world",
    "timeout_ms": 30000
  }'
```

### Resposta Tradicional

```json
{
  "error": "Use /api/v1/exec/stream endpoint for real-time execution",
  "recommended_endpoint": "/api/v1/exec/stream",
  "status": 422
}
```

### Health Check

```bash
curl https://codex-wrapper-hash.a.run.app/health
# Resposta: OK
```

## Monitoramento

### Logs

```bash
# Ver logs em tempo real
gcloud logging tail "resource.type=cloud_run_revision AND resource.labels.service_name=codex-wrapper"

# Filtrar erros
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=codex-wrapper AND severity=ERROR" --limit 50
```

### Métricas

```bash
# CPU utilization
gcloud monitoring metrics list --filter="metric.type:run.googleapis.com/container/cpu/utilizations"

# Request count
gcloud monitoring metrics list --filter="metric.type:run.googleapis.com/request_count"
```

## Custos Estimados

| Componente | Custo Mensal | Notas |
|------------|--------------|-------|
| Cloud Run | $20-100 | Baseado em requests e CPU time |
| Cloud Storage | $5-20 | 100GB de sessões |
| Cloud Load Balancer | $15 | Taxa fixa |
| Logs/Monitoring | $5-10 | Baseado em volume |
| **Total** | **$45-145** | Para 1000-10000 requests/mês |

## Vantagens

- ✅ **Setup simples** - Deploy em minutos
- ✅ **Custo baixo** - Pay-per-use
- ✅ **Escalabilidade automática** - 0 → 1000 instâncias
- ✅ **Manutenção mínima** - Managed service
- ✅ **HTTPS automático** - SSL/TLS incluído

## Limitações

- ❌ **Cold start** - 500ms-2s primeira request
- ❌ **Stateless** - Sem persistência entre requests
- ❌ **Timeout máximo** - 60 minutos por request
- ❌ **Sem session continuity** - Cada request é isolada

## Melhorias Futuras

1. **Cache Redis** - Para sessões frequentes
2. **CDN** - Para assets estáticos
3. **WebSocket** - Para streaming real-time
4. **Background jobs** - Para tarefas longas
5. **Auth** - API keys e rate limiting

## Troubleshooting

### Problemas Comuns

1. **Timeout na execução**
   ```bash
   # Aumentar timeout
   gcloud run services update codex-wrapper --timeout=3600
   ```

2. **Memória insuficiente**
   ```bash
   # Aumentar memória
   gcloud run services update codex-wrapper --memory=4Gi
   ```

3. **Muitas instâncias**
   ```bash
   # Limitar concorrência
   gcloud run services update codex-wrapper --concurrency=10
   ```

### Debug Local

```bash
# Testar localmente
export BUCKET_NAME=test-bucket
export PORT=8080
cargo run

# Testar endpoint SSE
curl -N -X POST localhost:8080/api/v1/exec/stream \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{"prompt": "echo hello"}'

# Testar endpoint tradicional (deve retornar erro)
curl -X POST localhost:8080/api/v1/exec \
  -H "Content-Type: application/json" \
  -d '{"prompt": "echo hello"}'
```

## Análise de Dependências do Codex-RS

### **Dependências Principais Requeridas**

**Core Dependencies (Cargo.toml):**
```toml
[dependencies]
# Cloud Run wrapper específico
axum = "0.8"
google-cloud-storage = "0.24"
google-cloud-firestore = "0.54"
uuid = { version = "1", features = ["v4", "serde", "v7"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }

# Compatibilidade com codex-rs
tokio = { version = "1", features = ["io-std", "macros", "process", "rt-multi-thread", "signal"] }
anyhow = "1"
tracing = { version = "0.1.41", features = ["log"] }
tracing-subscriber = { version = "0.3.20", features = ["env-filter", "fmt"] }
```

**Codex Internal Crates (do workspace):**
```toml
codex-app-server = { path = "../codex-rs/app-server" }
codex-common = { path = "../codex-rs/common", features = ["cli"] }
codex-core = { path = "../codex-rs/core" }
codex-protocol = { path = "../codex-rs/protocol" }
codex-arg0 = { path = "../codex-rs/arg0" }
```

### **Arquitetura de Funcionamento do codex-app-server**

**Protocolo de Comunicação:**
- **Entrada:** stdin (JSON-RPC 2.0 linha por linha)
- **Saída:** stdout (JSON-RPC responses linha por linha)
- **Métodos suportados:** `initialize`, `execOneOffCommand`, etc.

**Concorrência Interna (3 tasks assíncronas):**
```rust
// Task 1: stdin reader → canal incoming
tokio::spawn(async move {
    let mut lines = BufReader::new(stdin()).lines();
    while let Some(line) = lines.next_line().await {
        let msg: JSONRPCMessage = serde_json::from_str(&line)?;
        incoming_tx.send(msg).await?;
    }
});

// Task 2: message processor → processa requests
tokio::spawn(async move {
    while let Some(msg) = incoming_rx.recv().await {
        processor.process_request(msg).await;
    }
});

// Task 3: stdout writer → envia responses
tokio::spawn(async move {
    while let Some(msg) = outgoing_rx.recv().await {
        stdout().write_all(serde_json::to_string(&msg)?.as_bytes()).await?;
    }
});
```

### **Configurações Críticas para Aguardar Resposta**

**1. Timeout Management:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "execOneOffCommand", 
  "params": {
    "command": ["create a python script"],
    "timeoutMs": 30000
  }
}
```

**2. Process Lifecycle no Wrapper:**
```rust
// Spawn codex-app-server
let mut child = Command::new("./codex-app-server")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped()) 
    .stderr(Stdio::piped())
    .spawn()?;

// Comunicação JSON-RPC
let mut stdin = child.stdin.take().unwrap();
stdin.write_all(json_request.as_bytes()).await?;

// Aguardar conclusão com timeout
let output = child.wait_with_output().await?;
```

**3. Configurações Cloud Run Específicas:**
```bash
# Configurações mínimas para codex-rs
gcloud run deploy codex-wrapper \
  --memory 2Gi \                  # Mínimo para Rust + processamento
  --cpu 2 \                       # Para concorrência tokio (3 tasks)
  --timeout=3600 \                # Para comandos de AI longos
  --concurrency=10 \              # Limitar instâncias simultâneas
  --max-instances 100 \           # Escalabilidade automática
  --set-env-vars RUST_LOG=info,BUCKET_NAME=codex-sessions
```

**4. Variáveis de Ambiente Necessárias:**
```bash
# Essenciais para codex-app-server
export RUST_LOG=info                    # Controle de tracing
export BUCKET_NAME=codex-sessions-$PROJECT_ID
export PORT=8080

# Opcionais (com defaults sensatos)
export CODEX_TIMEOUT_MS=30000           # Timeout padrão
export CODEX_MAX_RETRIES=3              # Tentativas de retry
export CODEX_LOG_LEVEL=info             # Nível de log específico
```

### **Adaptações Necessárias no Wrapper**

**1. Adaptação de E/O (HTTP ↔ stdin/stdout):**
```rust
// HTTP Request → JSON-RPC stdin
let init_cmd = json!({
    "jsonrpc": "2.0",
    "id": 1, 
    "method": "initialize",
    "params": {"clientInfo": {"name": "cloud-wrapper", "version": "1.0.0"}}
});

let exec_cmd = json!({
    "jsonrpc": "2.0",
    "id": 2,
    "method": "execOneOffCommand", 
    "params": {
        "command": [req.prompt],
        "timeoutMs": req.timeout_ms.unwrap_or(30000)
    }
});

// Envio sequencial para stdin
stdin.write_all(format!("{}\n{}\n", init_cmd, exec_cmd).as_bytes()).await?;
```

**2. Gestão de Estado/Sessão:**
```rust
// Cada request HTTP = nova instância codex-app-server
// Estado persistido no Cloud Storage
let session_data = json!({
    "session_id": uuid::Uuid::new_v4(),
    "prompt": req.prompt,
    "stdout": output.stdout,
    "stderr": output.stderr, 
    "exit_code": output.status.code(),
    "timestamp": chrono::Utc::now()
});

save_session_to_storage(&state, &session_id, &session_data).await?;
```

**3. Parser de Responses JSON-RPC e Destinos de Resposta:**
```rust
// PROBLEMA: Modelo atual só pega stdout/stderr bruto
// let output = child.wait_with_output().await?;  ❌

// SOLUÇÃO: Parse linha por linha das responses JSON-RPC
let mut stdout_reader = BufReader::new(child.stdout.take().unwrap());
let mut stderr_reader = BufReader::new(child.stderr.take().unwrap());
let mut stdout_lines = stdout_reader.lines();

let mut final_result = String::new();
let mut task_files_created = Vec::new();
let mut task_progress = Vec::new();

// Parse responses JSON-RPC em tempo real
while let Some(line) = stdout_lines.next_line().await? {
    if let Ok(json_msg) = serde_json::from_str::<JSONRPCMessage>(&line) {
        match json_msg {
            JSONRPCMessage::Response(resp) => {
                if resp.id == 2 {  // execOneOffCommand response
                    if let Some(result) = resp.result {
                        final_result = result.to_string();
                        
                        // Extrair arquivos criados/modificados
                        if let Ok(exec_result) = serde_json::from_value::<ExecResult>(result) {
                            task_files_created = exec_result.files_created;
                            task_progress = exec_result.steps_completed;
                        }
                    }
                }
            },
            JSONRPCMessage::Notification(notif) => {
                // Streaming de progresso em tempo real
                if notif.method == "task/progress" {
                    task_progress.push(notif.params);
                }
            },
            _ => {}
        }
    }
}

child.wait().await?;  // Aguardar finalização do processo
```

**4. Destinos Finais da Resposta (Multi-canal):**

```rust
#[derive(Serialize)]
struct ExecResponse {
    session_id: String,
    exit_code: i32,
    
    // 📊 RESULTADO ESTRUTURADO (do JSON-RPC)
    task_result: TaskResult,           // Resultado principal da tarefa
    files_created: Vec<String>,        // Arquivos criados/modificados
    steps_completed: Vec<String>,      // Passos executados
    
    // 🔧 METADADOS TÉCNICOS
    execution_time_ms: u64,
    stdout_raw: String,                // stdout bruto (debug)
    stderr_raw: String,                // stderr bruto (debug)
}

#[derive(Serialize)]
struct TaskResult {
    success: bool,
    summary: String,                   // Resumo do que foi feito
    artifacts: Vec<Artifact>,          // Arquivos, outputs, links
    next_steps: Option<Vec<String>>,   // Sugestões de próximos passos
}

#[derive(Serialize)]  
struct Artifact {
    artifact_type: String,             // "file", "url", "command", "code"
    name: String,
    content: Option<String>,           // Conteúdo se disponível
    path: Option<String>,              // Caminho no filesystem
    metadata: Option<serde_json::Value>
}
```

**5. Canais de Distribuição da Resposta:**

```rust
// Canal 1: HTTP Response imediata (cliente web/API)
Ok(Json(ExecResponse {
    session_id,
    task_result,
    files_created,
    // ... resto dos campos
}))

// Canal 2: Cloud Storage detalhado (persistência)
let detailed_session = json!({
    "session_id": session_id,
    "prompt": req.prompt,
    "task_result": task_result,
    "artifacts": artifacts,
    "execution_timeline": task_progress,
    "metadata": {
        "timestamp": chrono::Utc::now(),
        "execution_time_ms": execution_time,
        "cloud_run_instance": std::env::var("K_REVISION"),
        "request_trace_id": trace_id
    }
});
save_session_to_storage(&state, &session_id, &detailed_session).await?;

// Canal 3: Real-time WebSocket (para UIs em tempo real)
if let Some(ws_broadcaster) = &state.websocket_broadcaster {
    ws_broadcaster.send(json!({
        "type": "task_completed",
        "session_id": session_id,
        "summary": task_result.summary,
        "files_created": files_created
    })).await?;
}

// Canal 4: Webhook/Callback (integrações externas)
if let Some(callback_url) = req.callback_url {
    let callback_payload = json!({
        "session_id": session_id,
        "status": "completed",
        "result": task_result,
        "webhook_timestamp": chrono::Utc::now()
    });
    
    tokio::spawn(async move {
        let _ = reqwest::Client::new()
            .post(&callback_url)
            .json(&callback_payload)
            .send()
            .await;
    });
}

// Canal 5: Metrics/Analytics (observabilidade)
metrics::counter!("codex_tasks_completed").increment(1);
metrics::histogram!("codex_execution_time_ms").record(execution_time as f64);
if task_result.success {
    metrics::counter!("codex_tasks_success").increment(1);
} else {
    metrics::counter!("codex_tasks_failed").increment(1);
}
```

**6. Error Handling e Recovery:**
```rust
// Timeout no nível do processo com cleanup
let output = timeout(
    Duration::from_millis(req.timeout_ms.unwrap_or(30000)),
    parse_jsonrpc_responses(child)
).await;

match output {
    Ok(Ok(result)) => result,
    Ok(Err(e)) => {
        // Erro durante parse JSON-RPC
        error!("JSON-RPC parse error: {}", e);
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    },
    Err(_) => {
        // Timeout - kill process e retornar parcial
        let _ = child.kill().await;
        return Ok(Json(ExecResponse {
            task_result: TaskResult {
                success: false,
                summary: "Task timed out".to_string(),
                artifacts: vec![],
                next_steps: Some(vec!["Try with longer timeout".to_string()])
            },
            // ... resto dos campos com valores padrão
        }));
    }
}
```

## Conclusão

Cloud Run é ideal para começar com wrapper cloud para codex-rs. Oferece simplicidade, baixo custo e escalabilidade automática, perfeito para validação de conceito e cargas médias.

**Compatibilidade confirmada:** O `codex-app-server` já implementa comunicação JSON-RPC assíncrona via stdin/stdout, tornando-o totalmente compatível com o wrapper Cloud Run proposto.

**Próximo passo:** Migrar para GKE quando precisar de performance máxima ou sessões persistentes.