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
use std::path::PathBuf;
use tokio::fs;

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

/// Upload de arquivo individual para Google Cloud Storage
pub async fn upload_file_to_gcs(bucket: &str, file_path: &str, object_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::process::Command;
    
    // Usa gsutil para fazer upload (mais confiável que a biblioteca)
    let output = Command::new("gsutil")
        .arg("cp")
        .arg(file_path)
        .arg(&format!("gs://{}/{}", bucket, object_name))
        .output()?;
    
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gsutil failed: {}", error_msg).into());
    }
    
    tracing::info!("Arquivo {} enviado com sucesso para gs://{}/{}", file_path, bucket, object_name);
    Ok(())
}

/// Detecta e envia arquivos criados no diretório de trabalho
pub async fn upload_created_files(session_id: &str, work_dir: &str) -> Vec<String> {
    let bucket = match env::var("GCS_FILES_BUCKET") {
        Ok(b) => b,
        Err(_) => {
            tracing::debug!("GCS_FILES_BUCKET não definida - upload de arquivos desabilitado");
            return vec![];
        }
    };
    
    let mut uploaded_files = Vec::new();
    
    // Lista todos os arquivos no diretório de trabalho
    if let Ok(mut entries) = fs::read_dir(work_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    let file_path = entry.path();
                    let file_name = file_path.file_name().unwrap().to_string_lossy();
                    
                    // Pula arquivos temporários e de sistema
                    if file_name.starts_with('.') || file_name.ends_with(".tmp") {
                        continue;
                    }
                    
                    let object_name = format!("files/{}/{}", session_id, file_name);
                    let file_path_str = file_path.to_string_lossy();
                    
                    match upload_file_to_gcs(&bucket, &file_path_str, &object_name).await {
                        Ok(_) => {
                            uploaded_files.push(format!("gs://{}/{}", bucket, object_name));
                        },
                        Err(e) => {
                            tracing::error!("Falha ao enviar arquivo {}: {}", file_path_str, e);
                        }
                    }
                }
            }
        }
    }
    
    uploaded_files
}

pub async fn save_session_to_storage(mut session: SessionPersistData) {
    let bucket = match env::var("GCS_SESSION_BUCKET") {
        Ok(b) => b,
        Err(_) => {
            tracing::debug!("GCS_SESSION_BUCKET não definida - persistência desabilitada");
            return;
        }
    };

    let object_name = format!(
        "sessions/{}-{}.json",
        session.session_id,
        session.timestamp.to_rfc3339()
    );
    
    // Upload dos arquivos criados primeiro
    let uploaded_files = upload_created_files(&session.session_id, "/tmp").await;
    session.created_files = if uploaded_files.is_empty() {
        None
    } else {
        Some(uploaded_files)
    };
    
    let json_data = match serde_json::to_vec_pretty(&session) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Falha ao serializar sessão para JSON: {:?}", e);
            return;
        }
    };
    
    // Salva temporariamente para upload
    let temp_file = format!("/tmp/session-{}.json", session.session_id);
    if let Err(e) = fs::write(&temp_file, &json_data).await {
        tracing::error!("Falha ao escrever arquivo temporário: {:?}", e);
        return;
    }
    
    // Upload da sessão
    match upload_file_to_gcs(&bucket, &temp_file, &object_name).await {
        Ok(_) => {
            tracing::info!("Sessão persistida em gs://{}/{}", bucket, object_name);
            // Remove arquivo temporário
            let _ = fs::remove_file(&temp_file).await;
        },
        Err(e) => {
            tracing::error!("Falha ao persistir sessão: {:?}", e);
        }
    }
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
    let model = req
        .model
        .clone()
        .unwrap_or_else(|| "gpt-4.1-mini".to_string());

    // Spawn subprocesso em task separada com timeout e kill garantido
    tokio::spawn({
        let tx = tx.clone();
        let prompt = prompt.clone();
        let session_id = session_id.clone();
        let model = model.clone();
        async move {
            use std::sync::Arc;
            use tokio::process::Child;
            use tokio::sync::Mutex;

            // Wrapper para compartilhar o processo
            let child_ref = Arc::new(Mutex::new(None::<Child>));
            let child_ref_clone = child_ref.clone();

            // Função auxiliar para encontrar o binário codex
            fn find_app_server_binary() -> Option<String> {
                use std::path::PathBuf;

                // 1. Tenta encontrar no mesmo diretório do executável atual
                if let Ok(exe_path) = std::env::current_exe() {
                    if let Some(exe_dir) = exe_path.parent() {
                        // Procura por "codex" em vez de "codex-app-server"
                        let candidate = exe_dir.join("codex");
                        if candidate.exists() {
                            tracing::info!("Found codex at: {:?}", candidate);
                            return Some(candidate.display().to_string());
                        }
                    }
                }

                // 2. Tenta caminhos relativos ao diretório de trabalho atual
                let candidates = vec![
                    PathBuf::from("./codex"),
                    PathBuf::from("../codex-cli/bin/codex-x86_64-unknown-linux-musl"),
                ];

                for path in &candidates {
                    if path.exists() {
                        tracing::info!("Found codex at: {:?}", path);
                        if let Ok(canonical) = path.canonicalize() {
                            return Some(canonical.display().to_string());
                        }
                    }
                }

                // 3. Tenta no PATH
                tracing::warn!("codex not found in standard locations, trying PATH");
                Some("codex".to_string())
            }

            // Função modificada para salvar o child
            async fn run_process_with_ref(
                prompt: String,
                model: String,
                timeout_ms: u64,
                session_id: String,
                tx: mpsc::UnboundedSender<Event>,
                child_ref: Arc<Mutex<Option<Child>>>,
                allow_network: bool,
                allow_file_operations: bool,
                approval_policy: String,
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
                                .data("codex binary not found"),
                        );
                        return;
                    }
                };

                // Spawna o codex no modo proto
                // O modo proto permite comunicação via stdin/stdout
                let mut cmd = Command::new(&app_server_path);

                // Adiciona o comando "proto" para usar o modo de protocolo
                cmd.arg("proto");

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

                // Envia comando no formato Submission esperado pelo codex proto
                // Usa UserTurn para especificar o modelo e outras configurações
                let mut op = serde_json::Map::new();
                op.insert("type".to_string(), json!("user_turn"));
                op.insert("items".to_string(), json!([
                    {
                        "type": "text",
                        "text": prompt
                    }
                ]));
                op.insert("cwd".to_string(), json!("/tmp"));
                
                // Usa os parâmetros recebidos para configurar políticas
                op.insert("approval_policy".to_string(), json!(approval_policy));
                op.insert("sandbox_policy".to_string(), json!({
                    "mode": "danger-full-access",
                    "allow_network": allow_network,
                    "allow_file_operations": allow_file_operations
                }));
                op.insert("model".to_string(), json!(model));
                // Usar "medium" como valor padrão para effort
                op.insert("effort".to_string(), json!("medium"));
                op.insert("summary".to_string(), json!("auto"));
                op.insert("final_output_json_schema".to_string(), serde_json::Value::Null);

                let submission = json!({
                    "id": "req-1",
                    "op": serde_json::Value::Object(op)
                });

                tracing::info!("Sending submission: {}", submission);

                if let Err(e) = stdin
                    .write_all(format!("{}\n", submission).as_bytes())
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
                            // Parse eventos do protocolo codex
                            // O codex proto retorna Events com id e msg (EventMsg)
                            match serde_json::from_str::<serde_json::Value>(&line) {
                                Ok(json_msg) => {
                                    // Verifica se é um Event do protocolo codex
                                    if let Some(msg_type) = json_msg.get("msg")
                                        .and_then(|m| m.get("type"))
                                        .and_then(|t| t.as_str()) {

                                        // Manda o evento como está para o cliente
                                        let _ = tx.send(
                                            Event::default()
                                                .event(msg_type)
                                                .data(json_msg.to_string()),
                                        );

                                        // Eventos especiais que queremos tratar
                                        match msg_type {
                                            "agent_message" => {
                                                let _ = tx.send(
                                                    Event::default()
                                                        .event("agent_output")
                                                        .data(json_msg.to_string()),
                                                );
                                            }
                                            "task_complete" => {
                                                let _ = tx.send(
                                                    Event::default()
                                                        .event("task_completed")
                                                        .data(json_msg.to_string()),
                                                );
                                            }
                                            "error" => {
                                                let _ = tx.send(
                                                    Event::default()
                                                        .event("error")
                                                        .data(json_msg.to_string()),
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Não é JSON, enviar como linha de texto simples
                                    // Importante: nem todas as linhas são JSON válido
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

            // Extrai configurações da requisição para manter compatibilidade
            let allow_network = req.allow_network.unwrap_or(true);
            let allow_file_operations = req.allow_file_operations.unwrap_or(true);
            // Valores válidos para approval_policy: "untrusted", "on-failure", "on-request", "never"
            // Usando "never" = nunca pedir aprovação, executar tudo automaticamente
            let approval_policy = req.approval_policy.as_deref().unwrap_or("never").to_string();
            
            let process_fut = run_process_with_ref(
                prompt.clone(),
                model.clone(),
                timeout_ms,
                session_id.clone(),
                tx.clone(),
                child_ref_clone,
                allow_network,
                allow_file_operations,
                approval_policy,
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
