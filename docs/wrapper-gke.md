# ⚙️ GKE - Wrapper High-Performance

## Visão Geral

Wrapper de alta performance para codex-rs usando Google Kubernetes Engine (GKE). Ideal para cargas altas, baixa latência e necessidade de sessões persistentes.

## Arquitetura

```
Internet → GCP Load Balancer → GKE Ingress
                                   ↓
           ┌─── Gateway Service (Rust) ───┐
           │                              │
    ┌──────▼──────┐              ┌───────▼──────┐
    │ Codex Pool  │              │ Session Mgr  │
    │ (StatefulSet)│              │ (Deployment) │
    └─────────────┘              └──────────────┘
           │                              │
    ┌──────▼──────┐              ┌───────▼──────┐
    │ Redis Cache │              │Cloud Storage │
    │(MemoryStore)│              │   & Firestore │
    └─────────────┘              └──────────────┘
```

## Serviços GCP

- **GKE Autopilot** - Kubernetes gerenciado
- **Cloud Load Balancing** - L7 load balancer  
- **Cloud MemoryStore Redis** - Cache de sessões
- **Cloud Storage** - Persistent file storage
- **Cloud Monitoring** - Observability
- **Cloud Pub/Sub** - Async messaging
- **Workload Identity** - Autenticação segura

## Stack Rust

### Dependências (Cargo.toml)

```toml
[dependencies]
axum = { version = "0.8", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
redis = { version = "0.24", features = ["tokio-comp"] }
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = "0.3"
reqwest = { version = "0.11", features = ["json"] }
futures-util = "0.3"
dashmap = "5.0"
```

## Implementação

### Gateway Service (gateway/src/main.rs)

```rust
use axum::{
    extract::{Path, State, ws::{WebSocket, WebSocketUpgrade}},
    response::Response,
    routing::{get, post},
    Router, Json,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use std::{collections::HashMap, sync::Arc};
use dashmap::DashMap;

#[derive(Clone)]
struct AppState {
    redis_client: redis::Client,
    worker_pool: Arc<WorkerPool>,
    sessions: Arc<DashMap<String, SessionInfo>>,
}

struct WorkerPool {
    workers: Vec<WorkerNode>,
    current: std::sync::atomic::AtomicUsize,
}

#[derive(Clone)]
struct WorkerNode {
    id: String,
    endpoint: String,
    health: Arc<std::sync::atomic::AtomicBool>,
}

impl WorkerPool {
    async fn get_available_worker(&self) -> Option<&WorkerNode> {
        let start = self.current.load(std::sync::atomic::Ordering::Relaxed);
        
        for i in 0..self.workers.len() {
            let idx = (start + i) % self.workers.len();
            let worker = &self.workers[idx];
            
            if worker.health.load(std::sync::atomic::Ordering::Relaxed) {
                self.current.store((idx + 1) % self.workers.len(), std::sync::atomic::Ordering::Relaxed);
                return Some(worker);
            }
        }
        
        None
    }
}

#[derive(Deserialize)]
struct ExecRequest {
    prompt: String,
    session_id: Option<String>,
    timeout_ms: Option<u64>,
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone)]
struct SessionInfo {
    id: String,
    worker_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_accessed: chrono::DateTime<chrono::Utc>,
}

async fn exec_handler(
    State(state): State<AppState>,
    Json(req): Json<ExecRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let session_id = req.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    // Get or assign worker
    let worker = if let Some(session) = state.sessions.get(&session_id) {
        // Use existing worker for session continuity
        state.worker_pool.workers.iter()
            .find(|w| w.id == session.worker_id)
            .ok_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        // Get available worker
        state.worker_pool.get_available_worker()
            .ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?
    };
    
    // Execute on worker
    let result = execute_on_worker(worker, &req.prompt, req.timeout_ms).await?;
    
    // Update session info
    let session_info = SessionInfo {
        id: session_id.clone(),
        worker_id: worker.id.clone(),
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
    };
    
    state.sessions.insert(session_id.clone(), session_info.clone());
    
    // Cache in Redis
    if let Ok(mut redis_conn) = state.redis_client.get_async_connection().await {
        let _: Result<(), _> = redis_conn.setex(
            format!("session:{}", session_id),
            3600, // 1 hour TTL
            serde_json::to_string(&session_info).unwrap_or_default()
        ).await;
    }
    
    Ok(Json(result))
}

async fn execute_on_worker(
    worker: &WorkerNode,
    prompt: &str,
    timeout_ms: Option<u64>,
) -> Result<serde_json::Value, axum::http::StatusCode> {
    let client = reqwest::Client::new();
    
    let request_body = serde_json::json!({
        "prompt": prompt,
        "timeout_ms": timeout_ms.unwrap_or(30000)
    });
    
    let response = client
        .post(&format!("{}/execute", worker.endpoint))
        .json(&request_body)
        .timeout(std::time::Duration::from_millis(timeout_ms.unwrap_or(30000) + 5000))
        .send()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if response.status().is_success() {
        response.json().await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    } else {
        Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// WebSocket handler for streaming
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    while let Some(msg) = receiver.recv().await {
        if let Ok(msg) = msg {
            if let Ok(text) = msg.to_text() {
                if let Ok(req) = serde_json::from_str::<ExecRequest>(text) {
                    // Stream execution results back to client
                    if let Some(worker) = state.worker_pool.get_available_worker().await {
                        tokio::spawn(stream_execution(worker.clone(), req, sender.clone()));
                    }
                }
            }
        }
    }
}

async fn stream_execution(
    worker: WorkerNode,
    req: ExecRequest,
    mut sender: futures_util::stream::SplitSink<WebSocket, axum::extract::ws::Message>,
) {
    // Implementation for streaming execution results
    // This would connect to worker and stream results back to client
}

async fn health_handler() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    
    let redis_url = std::env::var("REDIS_URL")?;
    let redis_client = redis::Client::open(redis_url)?;
    
    // Initialize worker pool from environment/discovery
    let worker_endpoints: Vec<String> = std::env::var("CODEX_WORKERS")
        .unwrap_or_default()
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
        current: std::sync::atomic::AtomicUsize::new(0),
    });
    
    // Start health checking
    tokio::spawn(health_check_workers(worker_pool.clone()));
    
    let state = AppState {
        redis_client,
        worker_pool,
        sessions: Arc::new(DashMap::new()),
    };
    
    let app = Router::new()
        .route("/api/v1/exec", post(exec_handler))
        .route("/ws", get(ws_handler))
        .route("/health", get(health_handler))
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("Gateway server running on port 8080");
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn health_check_workers(pool: Arc<WorkerPool>) {
    let client = reqwest::Client::new();
    
    loop {
        for worker in &pool.workers {
            let health_result = client
                .get(&format!("{}/health", worker.endpoint))
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await;
            
            let is_healthy = health_result
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            
            worker.health.store(is_healthy, std::sync::atomic::Ordering::Relaxed);
        }
        
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}
```

### Worker Service (worker/src/main.rs)

```rust
use axum::{extract::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Deserialize)]
struct WorkerRequest {
    prompt: String,
    timeout_ms: Option<u64>,
}

#[derive(Serialize)]
struct WorkerResponse {
    exit_code: i32,
    stdout: String,
    stderr: String,
    execution_time_ms: u64,
}

struct CodexProcess {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
}

impl CodexProcess {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut child = tokio::process::Command::new("./codex-app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        let stdin = child.stdin.take().unwrap();
        
        // Initialize the process
        let init_cmd = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "gke-worker",
                    "version": "1.0.0"
                }
            }
        });
        
        Ok(Self { child, stdin })
    }
    
    async fn execute(&mut self, prompt: &str, timeout_ms: u64) -> Result<WorkerResponse, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        
        let exec_cmd = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "execOneOffCommand",
            "params": {
                "command": [prompt],
                "timeoutMs": timeout_ms
            }
        });
        
        self.stdin.write_all(format!("{}\n", exec_cmd).as_bytes()).await?;
        self.stdin.flush().await?;
        
        // Read response (simplified - in practice would parse JSON-RPC response)
        let output = self.child.wait_with_output().await?;
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(WorkerResponse {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time_ms: execution_time,
        })
    }
}

#[derive(Clone)]
struct WorkerState {
    process: Arc<Mutex<Option<CodexProcess>>>,
    worker_id: String,
}

async fn execute_handler(
    axum::extract::State(state): axum::extract::State<WorkerState>,
    Json(req): Json<WorkerRequest>,
) -> Result<Json<WorkerResponse>, axum::http::StatusCode> {
    let mut process_guard = state.process.lock().await;
    
    if process_guard.is_none() {
        match CodexProcess::new().await {
            Ok(process) => *process_guard = Some(process),
            Err(_) => return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
    
    if let Some(process) = process_guard.as_mut() {
        match process.execute(&req.prompt, req.timeout_ms.unwrap_or(30000)).await {
            Ok(response) => Ok(Json(response)),
            Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn health_handler() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    
    let worker_id = std::env::var("WORKER_ID").unwrap_or_else(|_| "worker-0".to_string());
    
    let state = WorkerState {
        process: Arc::new(Mutex::new(None)),
        worker_id,
    };
    
    let app = Router::new()
        .route("/execute", post(execute_handler))
        .route("/health", get(health_handler))
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:7000").await?;
    tracing::info!("Worker server running on port 7000");
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

## Kubernetes Manifests

### Gateway Deployment

```yaml
# k8s/gateway-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: codex-gateway
  namespace: codex
spec:
  replicas: 3
  selector:
    matchLabels:
      app: codex-gateway
  template:
    metadata:
      labels:
        app: codex-gateway
    spec:
      serviceAccountName: codex-gateway-sa
      containers:
      - name: gateway
        image: gcr.io/PROJECT_ID/codex-gateway:latest
        ports:
        - containerPort: 8080
        env:
        - name: REDIS_URL
          value: "redis://redis-service:6379"
        - name: CODEX_WORKERS
          value: "http://codex-pool-0.codex-pool-service:7000,http://codex-pool-1.codex-pool-service:7000,http://codex-pool-2.codex-pool-service:7000"
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: codex-gateway-service
  namespace: codex
spec:
  selector:
    app: codex-gateway
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8080
  type: ClusterIP
```

### Worker StatefulSet

```yaml
# k8s/worker-statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: codex-pool
  namespace: codex
spec:
  serviceName: "codex-pool-service"
  replicas: 5
  selector:
    matchLabels:
      app: codex-pool
  template:
    metadata:
      labels:
        app: codex-pool
    spec:
      serviceAccountName: codex-worker-sa
      containers:
      - name: codex-worker
        image: gcr.io/PROJECT_ID/codex-worker:latest
        ports:
        - containerPort: 7000
        env:
        - name: WORKER_ID
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        resources:
          requests:
            memory: "2Gi"
            cpu: "1000m"
          limits:
            memory: "4Gi"
            cpu: "2000m"
        volumeMounts:
        - name: worker-storage
          mountPath: /tmp/codex-workspace
        livenessProbe:
          httpGet:
            path: /health
            port: 7000
          initialDelaySeconds: 15
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 7000
          initialDelaySeconds: 10
          periodSeconds: 15
  volumeClaimTemplates:
  - metadata:
      name: worker-storage
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 10Gi
      storageClassName: "premium-rwo"
---
apiVersion: v1
kind: Service
metadata:
  name: codex-pool-service
  namespace: codex
spec:
  clusterIP: None
  selector:
    app: codex-pool
  ports:
  - protocol: TCP
    port: 7000
    targetPort: 7000
```

### Redis Deployment

```yaml
# k8s/redis-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: redis
  namespace: codex
spec:
  replicas: 1
  selector:
    matchLabels:
      app: redis
  template:
    metadata:
      labels:
        app: redis
    spec:
      containers:
      - name: redis
        image: redis:7-alpine
        ports:
        - containerPort: 6379
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "200m"
        volumeMounts:
        - name: redis-data
          mountPath: /data
      volumes:
      - name: redis-data
        persistentVolumeClaim:
          claimName: redis-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: redis-service
  namespace: codex
spec:
  selector:
    app: redis
  ports:
  - protocol: TCP
    port: 6379
    targetPort: 6379
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: redis-pvc
  namespace: codex
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: "premium-rwo"
```

### Ingress

```yaml
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: codex-ingress
  namespace: codex
  annotations:
    kubernetes.io/ingress.class: "gce"
    kubernetes.io/ingress.global-static-ip-name: "codex-ip"
    networking.gke.io/managed-certificates: "codex-ssl-cert"
spec:
  rules:
  - host: codex-api.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: codex-gateway-service
            port:
              number: 80
---
apiVersion: networking.gke.io/v1
kind: ManagedCertificate
metadata:
  name: codex-ssl-cert
  namespace: codex
spec:
  domains:
  - codex-api.example.com
```

## Deploy e Configuração

### 1. Criar Cluster GKE

```bash
# Variáveis
export PROJECT_ID=your-project-id
export CLUSTER_NAME=codex-cluster
export REGION=us-central1

# Habilitar APIs
gcloud services enable \
  container.googleapis.com \
  containerregistry.googleapis.com \
  redis.googleapis.com

# Criar cluster GKE Autopilot
gcloud container clusters create-auto $CLUSTER_NAME \
  --region=$REGION \
  --project=$PROJECT_ID

# Obter credenciais
gcloud container clusters get-credentials $CLUSTER_NAME --region=$REGION
```

### 2. Configurar Workload Identity

```bash
# Criar namespace
kubectl create namespace codex

# Criar service accounts
kubectl create serviceaccount codex-gateway-sa --namespace codex
kubectl create serviceaccount codex-worker-sa --namespace codex

# Configurar Workload Identity
gcloud iam service-accounts create codex-gke-sa

kubectl annotate serviceaccount codex-gateway-sa \
  --namespace codex \
  iam.gke.io/gcp-service-account=codex-gke-sa@$PROJECT_ID.iam.gserviceaccount.com

gcloud iam service-accounts add-iam-policy-binding \
  --role roles/iam.workloadIdentityUser \
  --member "serviceAccount:$PROJECT_ID.svc.id.goog[codex/codex-gateway-sa]" \
  codex-gke-sa@$PROJECT_ID.iam.gserviceaccount.com
```

### 3. Criar Redis Instance

```bash
# Criar instância Redis
gcloud redis instances create codex-redis \
  --size=1 \
  --region=$REGION \
  --redis-version=redis_7_0
```

### 4. Build e Deploy Imagens

```bash
# Build gateway
cd gateway
gcloud builds submit --tag gcr.io/$PROJECT_ID/codex-gateway

# Build worker
cd ../worker
gcloud builds submit --tag gcr.io/$PROJECT_ID/codex-worker

# Deploy manifests
cd ../k8s
kubectl apply -f namespace.yaml
kubectl apply -f redis-deployment.yaml
kubectl apply -f worker-statefulset.yaml
kubectl apply -f gateway-deployment.yaml
kubectl apply -f ingress.yaml
```

### 5. Configurar Monitoramento

```bash
# Instalar Prometheus (opcional)
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm install prometheus prometheus-community/kube-prometheus-stack --namespace monitoring --create-namespace

# Verificar pods
kubectl get pods -n codex
kubectl get ingress -n codex
```

## Uso da API

### Executar Comando

```bash
curl -X POST https://codex-api.example.com/api/v1/exec \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "create a rust web server",
    "timeout_ms": 60000,
    "session_id": "my-session"
  }'
```

### WebSocket Streaming

```javascript
const ws = new WebSocket('wss://codex-api.example.com/ws');

ws.onopen = () => {
  ws.send(JSON.stringify({
    prompt: "create a complex application",
    stream: true
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Progress:', data);
};
```

## Monitoramento e Observabilidade

### Métricas Customizadas

```rust
use prometheus::{Counter, Histogram, register_counter, register_histogram};

lazy_static! {
    static ref REQUESTS_TOTAL: Counter = register_counter!(
        "codex_requests_total", "Total number of requests"
    ).unwrap();
    
    static ref REQUEST_DURATION: Histogram = register_histogram!(
        "codex_request_duration_seconds", "Request duration"
    ).unwrap();
}

// Em handlers
async fn exec_handler(req: ExecRequest) -> Result<Json<ExecResponse>, StatusCode> {
    let _timer = REQUEST_DURATION.start_timer();
    REQUESTS_TOTAL.inc();
    
    // ... lógica do handler
}
```

### Logs Estruturados

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(state))]
async fn exec_handler(
    State(state): State<AppState>,
    Json(req): Json<ExecRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        session_id = %req.session_id.as_deref().unwrap_or("new"),
        prompt_length = req.prompt.len(),
        "Processing execution request"
    );
    
    // ... lógica
}
```

## Custos Estimados

| Componente | Recursos | Custo Mensal |
|------------|----------|--------------|
| GKE Cluster | 3 nodes e2-standard-4 | $200 |
| Redis MemoryStore | 1GB | $25 |
| Persistent Disks | 50GB SSD | $40 |
| Load Balancer | Standard | $15 |
| Monitoring | Logs + métricas | $20 |
| **Total** |  | **$300** |

## Vantagens

- ✅ **Alta performance** - Latência <50ms
- ✅ **Sessões persistentes** - Workers dedicados
- ✅ **Escalabilidade** - Horizontal e vertical
- ✅ **Observabilidade** - Métricas detalhadas
- ✅ **Controle total** - Configuração customizada
- ✅ **Sem cold start** - Workers sempre ativos

## Limitações

- ❌ **Complexidade alta** - Requer expertise K8s
- ❌ **Custo fixo** - Paga por recursos reservados
- ❌ **Manutenção** - Updates, patches, monitoramento
- ❌ **Overhead** - Infraestrutura adicional

## Troubleshooting

### Verificar Status dos Pods

```bash
kubectl get pods -n codex
kubectl describe pod codex-gateway-xxx -n codex
kubectl logs codex-pool-0 -n codex
```

### Verificar Conectividade

```bash
# Testar Redis
kubectl exec -it codex-gateway-xxx -n codex -- redis-cli -h redis-service ping

# Testar workers
kubectl exec -it codex-gateway-xxx -n codex -- curl http://codex-pool-0.codex-pool-service:7000/health
```

### Debug de Performance

```bash
# Métricas de CPU/Memory
kubectl top pods -n codex

# Eventos do cluster
kubectl get events -n codex --sort-by='.lastTimestamp'
```

## Conclusão

GKE oferece máxima performance e controle para wrapper de produção do codex-rs. Ideal quando latência baixa e sessões persistentes são críticas, justificando a complexidade adicional.

**Próximo passo:** Considerar arquitetura híbrida para combinar benefícios de diferentes serviços GCP.