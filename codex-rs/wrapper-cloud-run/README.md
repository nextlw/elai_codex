# Codex Wrapper Cloud Run ☁️

> Wrapper HTTP para execução do Codex App Server em ambientes Cloud Run/Azure Container Apps, com suporte a streaming SSE, persistência opcional no Google Cloud Storage e onboarding rápido para desenvolvedores.

**Status**: ✅ **PRODUÇÃO** | **Versão**: 1.0.0 | **URL**: https://codex-wrapper-467992722695.us-central1.run.app

---

## 📚 Documentação Rápida

**NOVO USUÁRIO?** Comece aqui:

- 🚀 **[QUICK_START.md](./QUICK_START.md)** - Comece em 5 minutos com exemplos prontos
- 📘 **[GUIA_COMPLETO_USO.md](./GUIA_COMPLETO_USO.md)** - Documentação completa com clientes Python/JS
- 📊 **[RESUMO_IMPLEMENTACAO.md](./RESUMO_IMPLEMENTACAO.md)** - Visão técnica e arquitetura
- 📋 **[INDEX.md](./INDEX.md)** - Índice de toda documentação

**ARQUIVOS PRONTOS PARA USAR**:
- 🐍 [codex_cloud_client.py](./codex_cloud_client.py) - Cliente Python (✅ testado)
- 🟨 [codex-cloud-client.js](./codex-cloud-client.js) - Cliente JavaScript (✅ testado)

---

## Visão Geral

Este wrapper expõe uma API HTTP (Axum) para executar comandos via Codex App Server, permitindo integração fácil com Google Cloud Run, Azure Container Apps e testes locais simples. O endpoint principal suporta streaming SSE para respostas em tempo real.

- **Endpoints principais:**
  - `GET /health`: Healthcheck simples
  - `POST /api/v1/exec/stream`: Executa comando e retorna eventos SSE
  - `POST /api/v1/exec`: (legacy, retorna erro orientando usar `/stream`)

- **Arquitetura resumida:**
  ```
  [Request HTTP] → [codex-wrapper] → [codex-app-server subprocesso] → [SSE Response]
  ```

- **Persistência opcional:**
  Se a variável `GCS_SESSION_BUCKET` estiver definida, sessões serão salvas no Google Cloud Storage (⚠️ **atualmente desabilitado** - veja [Troubleshooting](#troubleshooting)).

---

## Build e Execução Local

### Pré-requisitos

- **Rust >= 1.90.0** (versão fixada no projeto via `rust-toolchain`)
- [Tokio](https://tokio.rs/) (async runtime)
- [Google Cloud SDK](https://cloud.google.com/sdk) (apenas para testes com GCS - opcional)
- [Azure CLI](https://learn.microsoft.com/cli/azure/) (apenas para deploy no Azure - opcional)
- **Docker** (recomendado para testes com sandbox funcional)

### Build local

```sh
cd codex-rs/wrapper-cloud-run
cargo build --release
```

O binário será gerado em `target/release/codex-wrapper`.

### Execução local (com app-server)

O wrapper precisa do binário `codex-app-server` e de credenciais de AI providers para funcionar. Siga os passos:

**0. Configure as credenciais (crie um arquivo `.env`):**

```sh
cp .env.example .env
# Edite o .env com suas API keys reais
```

Ou exporte manualmente:
```sh
export OPENAI_API_KEY="sk-proj-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

**1. Compile o app-server:**

```sh
cd /Users/williamduarte/NCMproduto/codex/codex-rs/app-server
cargo build --release
```

**2. Crie um link simbólico no diretório do wrapper:**

```sh
cd /Users/williamduarte/NCMproduto/codex/codex-rs/wrapper-cloud-run
ln -sf ../app-server/target/release/codex-app-server .
```

**3. Execute o wrapper:**

```sh
cd /Users/williamduarte/NCMproduto/codex/codex-rs/wrapper-cloud-run
cargo run --release
# Ou diretamente:
# ./target/release/codex-wrapper-cloud-run
```

O servidor estará disponível em `http://localhost:8080`.

> **Nota:** O wrapper procura `./codex-app-server` no diretório de execução (linha 100 de `process.rs`).

### Build e execução com Docker (recomendado para testes com sandbox)

O Docker usa Linux onde o sandbox funciona perfeitamente (Landlock/seccomp), diferente do macOS que tem limitações com Seatbelt.

**1. Build da imagem (a partir do diretório `codex-rs`):**

```sh
cd /Users/williamduarte/NCMproduto/codex/codex-rs
docker build -f wrapper-cloud-run/Dockerfile -t codex-wrapper:latest .
```

**2. Executar com credenciais do .env:**

```sh
docker run -p 8080:8080 \
  -e OPENAI_API_KEY="$(grep OPENAI_API_KEY wrapper-cloud-run/.env | cut -d'=' -f2-)" \
  -e ANTHROPIC_API_KEY="$(grep ANTHROPIC_API_KEY wrapper-cloud-run/.env | cut -d'=' -f2-)" \
  -e GATEWAY_API_KEY="$(grep GATEWAY_API_KEY wrapper-cloud-run/.env | cut -d'=' -f2-)" \
  codex-wrapper:latest
```

**3. Ou passar diretamente:**

```sh
docker run -p 8080:8080 \
  -e OPENAI_API_KEY="sk-proj-..." \
  -e ANTHROPIC_API_KEY="sk-ant-..." \
  -e GATEWAY_API_KEY="IxF3WoAB6IBrNJKrC/Jsr5yjt2bXHZkBSHFDBhcIVvc=" \
  codex-wrapper:latest
```

**4. Testar:**

```sh
# Healthcheck
curl http://localhost:8080/health

# Request autenticado com o CLI
cd wrapper-cloud-run/cli
./target/release/codex-wrapper-cli --api-key "IxF3WoAB6IBrNJKrC/Jsr5yjt2bXHZkBSHFDBhcIVvc=" echo "Hello from Docker"
```

> **💡 Vantagem do Docker:** O sandbox Linux (Landlock/seccomp) funciona perfeitamente, sem as limitações do Seatbelt do macOS.
>
> **📋 Requisitos de sandbox:**
> - Kernel Linux 5.13+ com Landlock habilitado (Debian Bookworm tem 6.1+)
> - libseccomp2 instalado (incluído no Dockerfile)
> - Para ambientes que não suportam Landlock/seccomp, configure restrições no Docker e use `CODEX_UNSAFE_ALLOW_NO_SANDBOX=1`
>
> Veja mais detalhes em [`docs/sandbox.md`](../../docs/sandbox.md)

### Variáveis de ambiente

#### Configuração do Gateway
- `PORT` (opcional): Porta HTTP (padrão: 8080, usado pelo Cloud Run/Azure)
- `GATEWAY_API_KEY` (⚠️ **recomendado para produção**): API Key para autenticar requests
  - Se não definida, o wrapper roda em **modo desenvolvimento** (sem autenticação)
  - Exemplo: `export GATEWAY_API_KEY=seu-token-secreto-aqui`
- `GCS_SESSION_BUCKET` (opcional): Nome do bucket GCS para persistência de sessões (⚠️ funcionalidade desabilitada temporariamente)
- `RUST_LOG` (opcional): Nível de log (ex: `info`, `debug`, `trace`)

#### Credenciais de AI Providers (repassadas ao codex-app-server)
- `ANTHROPIC_API_KEY`: Chave da API Anthropic (Claude)
- `OPENAI_API_KEY`: Chave da API OpenAI (GPT)
- `OPENROUTER_API_KEY`: Chave da API OpenRouter
- `GOOGLE_API_KEY`: Chave da API Google (Gemini)
- `CODEX_CONFIG_PATH` (opcional): Caminho para arquivo de configuração customizado

---

## Exemplos de Uso dos Endpoints

### Healthcheck

```sh
curl http://localhost:8080/health
# OK
```

### Execução com Streaming SSE

**Sem autenticação (modo dev):**
```sh
curl -N -X POST http://localhost:8080/api/v1/exec/stream \
  -H "Content-Type: application/json" \
  -d '{"prompt": "echo Hello World"}'
```

**Com autenticação (produção):**
```sh
curl -N -X POST http://localhost:8080/api/v1/exec/stream \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer SEU_GATEWAY_API_KEY" \
  -d '{"prompt": "echo Hello World"}'
```

- O retorno será um stream SSE com eventos como:
  - `stdout_line`, `stderr_line`, `task_progress`, `task_result`, `task_completed`, `error`

#### Exemplo de resposta SSE

```
event: stdout_line
data: Hello World

event: task_completed
data: {"session_id":"...","exit_code":0,"status":"completed", ...}
```

### Execução legacy (não recomendado)

```sh
curl -X POST http://localhost:8080/api/v1/exec \
  -H "Content-Type: application/json" \
  -d '{"prompt": "echo Hello"}'
```

- Retorna 422 e orienta usar `/api/v1/exec/stream`.

---

## Testes Locais

- **Execução simples:**  
  Use o exemplo curl acima para testar comandos shell.
- **Streaming SSE:**  
  Use `curl -N` ou ferramentas como [httpie](https://httpie.io/) para visualizar eventos em tempo real.
- **Persistência GCS:**  
  Defina `GCS_SESSION_BUCKET` para testar salvamento de sessões (requer autenticação GCP).

---

## Troubleshooting

- **Erro: "Failed to spawn process"**
  - **Causa:** O binário `codex-app-server` não foi encontrado.
  - **Solução:**
    1. Compile o app-server: `cd ../app-server && cargo build --release`
    2. Crie o symlink: `cd ../wrapper-cloud-run && ln -sf ../app-server/target/release/codex-app-server .`
    3. Verifique se o link existe: `ls -lh codex-app-server`
  - **Alternativa:** Adicione `codex-app-server` ao PATH ou modifique linha 100 de `process.rs` para usar caminho absoluto.

- **Warning: "GCS_SESSION_BUCKET não definida"**
  - Este warning aparece nos logs, mas **não impede o funcionamento**.
  - A persistência em GCS está temporariamente desabilitada (linha 63-65 em `process.rs`).
  - Para remover o warning, você pode:
    1. Definir a variável (mesmo sem efeito): `export GCS_SESSION_BUCKET=dummy`
    2. Aguardar implementação completa da integração GCS

- **Erro de ABI do rust-analyzer (proc macro)**
  - Execute: `cargo clean && cargo check`
  - Reinicie o rust-analyzer no VS Code: `Cmd+Shift+P` → "Rust Analyzer: Restart Server"
  - Certifique-se que está usando Rust 1.90.0: `rustc --version`

- **Erro: "sandbox denied exec" (macOS)**
  - **Causa:** O Seatbelt do macOS bloqueia execução de comandos externos por padrão
  - **Soluções:**
    1. **Recomendado:** Use Docker (Linux) onde o sandbox funciona sem restrições
    2. Execute o wrapper em Cloud Run/Azure Container Apps (produção)
    3. Para testes locais rápidos, desabilite o sandbox (não recomendado):
       ```sh
       export CODEX_DISABLE_SANDBOX=1
       cargo run --release
       ```

- **Porta em uso**
  - Defina `PORT` para outra porta disponível: `export PORT=3000`

- **Parsing de JSON**
  - O endpoint espera um JSON válido com campo `prompt` (string).

- **Timeout**
  - O campo `timeout_ms` pode ser enviado no JSON para limitar a execução (padrão: 30000ms).

---

## Deploy em Nuvem

### Google Cloud Run

1. **Build da imagem Docker:**

   ```sh
   docker build -t gcr.io/SEU_PROJETO/codex-wrapper-cloud-run .
   ```

2. **Push para o Container Registry:**

   ```sh
   docker push gcr.io/SEU_PROJETO/codex-wrapper-cloud-run
   ```

3. **Deploy no Cloud Run com credenciais:**

   ```sh
   gcloud run deploy codex-wrapper-cloud-run \
     --image gcr.io/SEU_PROJETO/codex-wrapper-cloud-run \
     --platform managed \
     --region us-central1 \
     --allow-unauthenticated \
     --set-env-vars RUST_LOG=info,GATEWAY_API_KEY=seu-token-aqui \
     --set-secrets ANTHROPIC_API_KEY=anthropic-key:latest,OPENAI_API_KEY=openai-key:latest
   ```

   > **⚠️ Segurança:** Use Google Secret Manager para armazenar API keys!

### Azure Container Apps

1. **Build e push para Azure Container Registry:**

   ```sh
   az acr build --registry SEU_REGISTRY \
     --image codex-wrapper-cloud-run:latest \
     --file Dockerfile .
   ```

2. **Criar secrets no Azure:**

   ```sh
   az containerapp secret set \
     --name codex-wrapper \
     --resource-group SEU_RG \
     --secrets gateway-api-key=SEU_TOKEN \
              anthropic-key=SUA_ANTHROPIC_KEY \
              openai-key=SUA_OPENAI_KEY
   ```

3. **Deploy no Azure Container Apps:**

   ```sh
   az containerapp create \
     --name codex-wrapper \
     --resource-group SEU_RG \
     --environment SEU_ENVIRONMENT \
     --image SEU_REGISTRY.azurecr.io/codex-wrapper-cloud-run:latest \
     --target-port 8080 \
     --ingress external \
     --env-vars RUST_LOG=info \
     --secrets gateway-api-key anthropic-key openai-key \
     --cpu 1 --memory 2Gi
   ```

> **⚠️ Segurança:**
> - **SEMPRE** use `GATEWAY_API_KEY` em produção
> - Armazene API keys em secret managers (Google Secret Manager / Azure Key Vault)
> - **NUNCA** commite API keys no código ou Dockerfiles

---

## Referência Avançada

Para detalhes avançados de arquitetura, formatos de eventos SSE, exemplos de payloads e troubleshooting aprofundado, consulte os seguintes documentos:

- [`docs/wrapper-cloud-run.md`](../docs/wrapper-cloud-run.md) - Documentação completa do wrapper
- [`CLAUDE.md`](../CLAUDE.md) - Arquitetura do sistema cloud-native completo

---

## Status Atual

| Componente | Status | Observação |
|------------|--------|------------|
| **API HTTP/SSE** | ✅ Funcional | Streaming em tempo real |
| **Spawn subprocess** | ✅ Funcional | codex-app-server com env vars |
| **Autenticação** | ✅ Implementado | API Key via `Authorization: Bearer` header |
| **Credenciais AI** | ✅ Implementado | Suporte a Anthropic, OpenAI, OpenRouter, Google |
| **Persistência GCS** | ⚠️ Desabilitado | Aguardando atualização da lib `cloud-storage` |
| **Deploy Cloud Run** | ✅ Pronto | Testado e funcional |
| **Deploy Azure** | ✅ Pronto | Testado e funcional |

---

## Licença

MIT. Veja o arquivo [LICENSE](../../LICENSE).
