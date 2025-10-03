//! Utilitários para spawn e comunicação com o subprocesso codex-app-server

use crate::types::ExecRequest;
use axum::response::sse::Event;
use futures::stream::Stream;
use futures::StreamExt;
use serde_json::json;
use std::convert::Infallible;
use std::pin::Pin;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub type SseEventStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

// --- Persistência em Cloud Storage ---
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::env;

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionPersistData {
    session_id: String,
    prompt: String,
    exit_code: i32,
    status: String,
    execution_time_ms: u64,
    stdout: Vec<String>,
    stderr: Vec<String>,
    created_files: Option<Vec<String>>,
    timestamp: DateTime<Utc>,
    metadata: serde_json::Value,
}

pub async fn save_session_to_storage(session: SessionPersistData) {
    let _bucket = match env::var("GCS_SESSION_BUCKET") {
        Ok(b) => b,
        Err(_) => {
            tracing::debug!("GCS_SESSION_BUCKET não definida - persistência desabilitada");
            return;
        }
    };

    let _object_name = format!(
        "sessions/{}-{}.json",
        session.session_id,
        session.timestamp.to_rfc3339()
    );
    let _json_data = match serde_json::to_vec_pretty(&session) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Falha ao serializar sessão para JSON: {:?}", e);
            return;
        }
    };
    // FIXME: Persistência GCS desativada temporariamente devido à ausência do método correto na cloud-storage v0.11
    tracing::debug!("Persistência em GCS desativada: método correto para upload não encontrado na versão atual da cloud-storage.");
}

/// Spawna o codex-app-server, envia comandos JSON-RPC e faz streaming SSE dos eventos.
pub async fn run_codex_app_server_stream(req: ExecRequest) -> SseEventStream {
    use tokio::time::timeout;
    use tokio::time::Duration;
    let (tx, rx) = mpsc::unbounded_channel();
    let prompt = req.prompt.clone();
    let timeout_ms = req.timeout_ms.unwrap_or(30_000);
    let session_id = req
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Spawn subprocesso em task separada com timeout e kill garantido
    tokio::spawn({
        let tx = tx.clone();
        let prompt = prompt.clone();
        let session_id = session_id.clone();
        async move {
            use std::sync::Arc;
            use tokio::process::Child;
            use tokio::sync::Mutex;

            // Wrapper para compartilhar o processo
            let child_ref = Arc::new(Mutex::new(None::<Child>));
            let child_ref_clone = child_ref.clone();

            // Função auxiliar para encontrar o binário codex-app-server
            fn find_app_server_binary() -> Option<String> {
                use std::path::PathBuf;

                // 1. Tenta encontrar no mesmo diretório do executável atual
                if let Ok(exe_path) = std::env::current_exe() {
                    if let Some(exe_dir) = exe_path.parent() {
                        let candidate = exe_dir.join("codex-app-server");
                        if candidate.exists() {
                            tracing::info!("Found codex-app-server at: {:?}", candidate);
                            return Some(candidate.display().to_string());
                        }
                    }
                }

                // 2. Tenta caminhos relativos ao diretório de trabalho atual
                let candidates = vec![
                    PathBuf::from("./codex-app-server"),
                    PathBuf::from("../app-server/target/release/codex-app-server"),
                ];

                for path in &candidates {
                    if path.exists() {
                        tracing::info!("Found codex-app-server at: {:?}", path);
                        if let Ok(canonical) = path.canonicalize() {
                            return Some(canonical.display().to_string());
                        }
                    }
                }

                // 3. Tenta no PATH
                tracing::warn!("codex-app-server not found in standard locations, trying PATH");
                Some("codex-app-server".to_string())
            }

            // Função modificada para salvar o child
            async fn run_process_with_ref(
                prompt: String,
                timeout_ms: u64,
                session_id: String,
                tx: mpsc::UnboundedSender<Event>,
                child_ref: Arc<Mutex<Option<Child>>>,
            ) {
                let start_time = std::time::Instant::now();

                let _ = tx.send(
                    Event::default().event("task_started").data(
                        json!({
                            "session_id": session_id,
                            "status": "initializing"
                        })
                        .to_string(),
                    ),
                );

                let app_server_path = match find_app_server_binary() {
                    Some(path) => path,
                    None => {
                        let _ = tx.send(
                            Event::default()
                                .event("error")
                                .data("codex-app-server binary not found"),
                        );
                        return;
                    }
                };

                // Spawna o codex-app-server
                // Sandbox bypass via SandboxPolicy::DangerFullAccess no JSON-RPC
                let mut cmd = Command::new(&app_server_path);

                cmd.stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());

                // Passa credenciais de AI providers
                if let Ok(val) = env::var("ANTHROPIC_API_KEY") {
                    cmd.env("ANTHROPIC_API_KEY", val);
                }
                if let Ok(val) = env::var("OPENAI_API_KEY") {
                    cmd.env("OPENAI_API_KEY", val);
                }
                if let Ok(val) = env::var("OPENROUTER_API_KEY") {
                    cmd.env("OPENROUTER_API_KEY", val);
                }
                if let Ok(val) = env::var("GOOGLE_API_KEY") {
                    cmd.env("GOOGLE_API_KEY", val);
                }

                // Passa configurações opcionais
                if let Ok(val) = env::var("CODEX_CONFIG_PATH") {
                    cmd.env("CODEX_CONFIG_PATH", val);
                }
                if let Ok(val) = env::var("RUST_LOG") {
                    cmd.env("RUST_LOG", val);
                }
                if let Ok(val) = env::var("CODEX_UNSAFE_ALLOW_NO_SANDBOX") {
                    cmd.env("CODEX_UNSAFE_ALLOW_NO_SANDBOX", val);
                }

                let child = match cmd.spawn() {
                    Ok(child) => child,
                    Err(e) => {
                        let _ = tx.send(
                            Event::default()
                                .event("error")
                                .data(format!("Failed to spawn process: {}", e)),
                        );
                        return;
                    }
                };
                // Salva referência ao processo para kill externo
                {
                    let mut locked = child_ref.lock().await;
                    *locked = Some(child);
                }

                // Recupera stdin, stdout, stderr
                let mut locked = child_ref.lock().await;
                let child = locked.as_mut().unwrap();
                let mut stdin = match child.stdin.take() {
                    Some(stdin) => stdin,
                    None => {
                        let _ =
                            tx.send(Event::default().event("error").data("Failed to open stdin"));
                        return;
                    }
                };

                let init_cmd = json!({
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
                let exec_cmd = json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "execOneOffCommand",
                    "params": {
                        "command": [prompt],
                        "timeoutMs": timeout_ms,
                        "sandboxPolicy": "DangerFullAccess"
                    }
                });

                tracing::info!("Sending init_cmd: {}", init_cmd);
                tracing::info!("Sending exec_cmd: {}", exec_cmd);

                if let Err(e) = stdin
                    .write_all(format!("{}\n{}\n", init_cmd, exec_cmd).as_bytes())
                    .await
                {
                    let _ = tx.send(
                        Event::default()
                            .event("error")
                            .data(format!("Failed to write to stdin: {}", e)),
                    );
                    return;
                }
                let _ = stdin.flush().await;

                // Preparar buffers para coleta de stdout/stderr
                let (stdout_tx, mut stdout_rx) = mpsc::unbounded_channel::<String>();
                let (stderr_tx, mut stderr_rx) = mpsc::unbounded_channel::<String>();
                let mut stdout_buffer = Vec::new();
                let mut stderr_buffer = Vec::new();

                // Leitura concorrente de stdout
                let stdout_reader = BufReader::new(child.stdout.take().unwrap());
                let _stdout_task = {
                    let stdout_tx = stdout_tx.clone();
                    tokio::spawn(async move {
                        let mut lines = stdout_reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let _ = stdout_tx.send(line);
                        }
                    })
                };

                // Leitura concorrente de stderr
                let stderr_reader = BufReader::new(child.stderr.take().unwrap());
                let _stderr_task = {
                    let stderr_tx = stderr_tx.clone();
                    tokio::spawn(async move {
                        let mut lines = stderr_reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let _ = stderr_tx.send(line);
                        }
                    })
                };

                // Processamento dos eventos das linhas
                loop {
                    tokio::select! {
                        Some(line) = stdout_rx.recv() => {
                            stdout_buffer.push(line.clone());
                            let _ = tx.send(Event::default().event("stdout_line").data(line.clone()));
                            match serde_json::from_str::<serde_json::Value>(&line) {
                                Ok(json_msg) => {
                                    if let Some(method) = json_msg.get("method").and_then(|m| m.as_str()) {
                                        if method == "task/progress" {
                                            let _ = tx.send(
                                                Event::default()
                                                    .event("task_progress")
                                                    .data(json_msg.to_string()),
                                            );
                                        }
                                    }
                                    if let Some(id) = json_msg.get("id").and_then(|v| v.as_i64()) {
                                        if id == 2 {
                                            if let Some(result) = json_msg.get("result") {
                                                let _ = tx.send(
                                                    Event::default()
                                                        .event("task_result")
                                                        .data(result.to_string()),
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(
                                        Event::default()
                                            .event("error")
                                            .data(json!({
                                                "session_id": session_id,
                                                "error": "json_parse",
                                                "message": format!("Erro ao fazer parsing de stdout: {}", e),
                                                "line": line
                                            }).to_string()),
                                    );
                                }
                            }
                        }
                        Some(line) = stderr_rx.recv() => {
                            stderr_buffer.push(line.clone());
                            let _ = tx.send(Event::default().event("stderr_line").data(line));
                        }
                        else => {
                            break;
                        }
                    }
                }

                // Espera finalização
                let mut locked = child_ref.lock().await;
                let child = locked.as_mut().unwrap();
                let exit_status = child.wait().await.ok();
                let execution_time = start_time.elapsed().as_millis() as u64;

                let _ = tx.send(Event::default()
                    .event("task_completed")
                    .data(json!({
                        "session_id": session_id,
                        "exit_code": exit_status.and_then(|s| s.code()).unwrap_or(-1),
                        "execution_time_ms": execution_time,
                        "status": if exit_status.map(|s| s.success()).unwrap_or(false) { "completed" } else { "failed" },
                        "stdout": stdout_buffer,
                        "stderr": stderr_buffer
                    }).to_string())
                );

                // Persistência Cloud Storage
                let persist_data = SessionPersistData {
                    session_id: session_id.clone(),
                    prompt: prompt.clone(),
                    exit_code: exit_status.and_then(|s| s.code()).unwrap_or(-1),
                    status: if exit_status.map(|s| s.success()).unwrap_or(false) {
                        "completed".to_string()
                    } else {
                        "failed".to_string()
                    },
                    execution_time_ms: execution_time,
                    stdout: stdout_buffer.clone(),
                    stderr: stderr_buffer.clone(),
                    created_files: None,
                    timestamp: Utc::now(),
                    metadata: json!({}),
                };
                tokio::spawn(save_session_to_storage(persist_data));
            }

            let process_fut = run_process_with_ref(
                prompt.clone(),
                timeout_ms,
                session_id.clone(),
                tx.clone(),
                child_ref_clone,
            );
            match timeout(Duration::from_millis(timeout_ms), process_fut).await {
                Ok(_) => { /* terminou normalmente */ }
                Err(_) => {
                    // Timeout atingido: kill garantido
                    let mut locked = child_ref.lock().await;
                    if let Some(child) = locked.as_mut() {
                        let _ = child.kill().await;
                    }
                    let _ = tx.send(
                        Event::default()
                            .event("error")
                            .data(json!({
                                "session_id": session_id,
                                "error": "timeout",
                                "message": format!("Subprocesso excedeu o tempo limite de {}ms e foi encerrado forçadamente", timeout_ms)
                            }).to_string()),
                    );

                    // Persistência Cloud Storage para timeout
                    let persist_data = SessionPersistData {
                        session_id: session_id.clone(),
                        prompt: prompt.clone(),
                        exit_code: -1,
                        status: "timeout".to_string(),
                        execution_time_ms: timeout_ms,
                        stdout: vec![],
                        stderr: vec![],
                        created_files: None,
                        timestamp: Utc::now(),
                        metadata: json!({ "error": "timeout" }),
                    };
                    tokio::spawn(save_session_to_storage(persist_data));
                }
            }
        }
    });

    Box::pin(UnboundedReceiverStream::new(rx).map(Ok))
}
