# Codex Wrapper CLI

CLI para interagir com o Codex Wrapper Cloud Run via SSE (Server-Sent Events).

## Build

```bash
cd /Users/williamduarte/NCMproduto/codex/codex-rs/wrapper-cloud-run/cli
cargo build --release
```

O binário será gerado em `target/release/codex-wrapper-cli`.

## Uso

### Modo Single-Command

Executa um único comando e sai:

```bash
./target/release/codex-wrapper-cli --api-key "SEU_TOKEN" echo "Hello World"
```

### Modo Interativo

Entre em modo REPL para executar múltiplos comandos:

```bash
./target/release/codex-wrapper-cli --api-key "SEU_TOKEN"
```

Ou carregue o token do .env:

```bash
# Crie um .env com GATEWAY_API_KEY=seu-token
export GATEWAY_API_KEY="IxF3WoAB6IBrNJKrC/Jsr5yjt2bXHZkBSHFDBhcIVvc="
./target/release/codex-wrapper-cli
```

No modo interativo:

```
🚀 Codex Cloud Wrapper - Modo Interativo
Conectado a: http://localhost:8080
Digite seus comandos (Ctrl+D ou 'exit' para sair)

codex> echo Hello
codex> ls -la
codex> exit
```

## Opções

```
Options:
  -u, --url <URL>          URL do wrapper [default: http://localhost:8080]
  -k, --api-key <API_KEY>  API Key para autenticação (Bearer token)
  -t, --timeout <TIMEOUT>  Timeout em segundos [default: 300]
  -h, --help               Print help
  -V, --version            Print version
```

## Variáveis de Ambiente (via .env)

O CLI carrega automaticamente variáveis do arquivo `.env`:

```bash
# cli/.env
GATEWAY_API_KEY=IxF3WoAB6IBrNJKrC/Jsr5yjt2bXHZkBSHFDBhcIVvc=
```

## Eventos SSE Suportados

O CLI processa os seguintes eventos SSE do wrapper:

- `task_started` - Task iniciada com session_id
- `stdout_line` - Saída padrão do comando
- `stderr_line` - Saída de erro do comando
- `task_progress` - Progresso da task
- `task_result` - Resultado da task
- `task_completed` - Task concluída com exit code
- `error` - Erro durante a execução

## Exemplos

### Conectar a um wrapper remoto

```bash
./target/release/codex-wrapper-cli \
  --url https://codex-wrapper.azurecontainerapps.io \
  --api-key "seu-token-producao" \
  "create a python script to calculate fibonacci"
```

### Timeout customizado

```bash
./target/release/codex-wrapper-cli \
  --api-key "SEU_TOKEN" \
  --timeout 600 \
  "run long task"
```

## Instalação Global (opcional)

```bash
# Copiar binário para PATH
sudo cp target/release/codex-wrapper-cli /usr/local/bin/codex-cloud

# Usar de qualquer lugar
codex-cloud --help
```

## Troubleshooting

### Erro 401 Unauthorized

- Verifique se a `GATEWAY_API_KEY` está correta
- Confirme que o wrapper está com autenticação habilitada

### Erro de conexão

- Verifique se o wrapper está rodando: `curl http://localhost:8080/health`
- Confirme a URL com `--url`

### Sandbox denied exec (macOS)

Este é um erro esperado do codex-app-server em macOS devido às restrições de sandbox. Para ambientes de produção, use Linux (Cloud Run/Azure Container Apps).
