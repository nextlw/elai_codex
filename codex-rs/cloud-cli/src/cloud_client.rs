use anyhow::{Context, Result};
use futures::StreamExt;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::{Command, Stdio};

const CLOUD_URL: &str = "https://codex-wrapper-467992722695.us-central1.run.app";

#[derive(Debug, Serialize)]
pub struct ExecRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub timeout_ms: Option<u64>,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SseEvent {
    pub event: String,
    pub data: Value,
}

pub struct CloudClient {
    base_url: String,
    token: String,
}

impl CloudClient {
    /// Cria um novo cliente cloud
    /// Automaticamente obtém o token do gcloud
    pub fn new() -> Result<Self> {
        let token = Self::get_gcloud_token()?;
        Ok(Self {
            base_url: CLOUD_URL.to_string(),
            token,
        })
    }

    /// Obtém token de autenticação do gcloud
    fn get_gcloud_token() -> Result<String> {
        let output = Command::new("gcloud")
            .args(&["auth", "print-identity-token"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Falha ao executar gcloud. Certifique-se de que está instalado e autenticado.")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Falha ao obter token do gcloud: {}\n\nExecute: gcloud auth login adm@nexcode.live",
                error
            );
        }

        let token = String::from_utf8(output.stdout)
            .context("Token do gcloud contém dados inválidos")?
            .trim()
            .to_string();

        if token.is_empty() {
            anyhow::bail!("Token vazio retornado pelo gcloud. Execute: gcloud auth login adm@nexcode.live");
        }

        Ok(token)
    }

    /// Verifica se está autenticado
    pub fn check_auth() -> Result<()> {
        Self::get_gcloud_token()?;
        Ok(())
    }

    /// Executa um prompt no cloud e retorna stream de eventos SSE
    pub async fn exec_stream(&self, request: ExecRequest) -> Result<impl futures::Stream<Item = Result<SseEvent>>> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/exec/stream", self.base_url);

        let response = client
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .json(&request)
            .send()
            .await
            .context("Falha ao conectar com o serviço cloud")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Erro HTTP {}: {}", status, error_text);
        }

        // Converte o stream de bytes em stream de eventos SSE
        let stream = response.bytes_stream().map(move |chunk_result| {
            chunk_result
                .context("Erro ao ler stream")
                .and_then(|chunk| {
                    let text = String::from_utf8(chunk.to_vec())
                        .context("Dados inválidos no stream")?;

                    // Parse SSE format
                    parse_sse_chunk(&text)
                })
        });

        Ok(stream)
    }

    /// Executa um prompt e retorna apenas a resposta final
    pub async fn exec_simple(&self, prompt: &str, model: Option<String>) -> Result<String> {
        let request = ExecRequest {
            prompt: prompt.to_string(),
            model,
            timeout_ms: Some(60000),
            session_id: None,
        };

        let mut stream = self.exec_stream(request).await?;
        let mut final_message = String::new();

        while let Some(event_result) = stream.next().await {
            let event = event_result?;

            // Captura mensagem final
            if matches!(event.event.as_str(), "agent_message" | "agent_output") {
                if let Some(message) = event.data.get("message").and_then(|v| v.as_str()) {
                    final_message = message.to_string();
                }
            } else if event.event == "task_complete" {
                if let Some(message) = event.data.get("last_agent_message").and_then(|v| v.as_str()) {
                    final_message = message.to_string();
                }
                break;
            } else if event.event == "error" {
                if let Some(error) = event.data.get("message").and_then(|v| v.as_str()) {
                    anyhow::bail!("Erro no cloud: {}", error);
                }
            }
        }

        Ok(final_message)
    }
}

/// Parse um chunk SSE e retorna o evento se completo
fn parse_sse_chunk(text: &str) -> Result<SseEvent> {
    let mut event_type = String::from("unknown");
    let mut data = Value::Null;

    for line in text.lines() {
        if line.starts_with("event:") {
            event_type = line[6..].trim().to_string();
        } else if line.starts_with("data:") {
            let data_str = line[5..].trim();
            data = serde_json::from_str(data_str)
                .unwrap_or_else(|_| Value::String(data_str.to_string()));
        }
    }

    Ok(SseEvent {
        event: event_type,
        data,
    })
}
