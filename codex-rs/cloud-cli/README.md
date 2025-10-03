# Codex Cloud CLI

CLI dedicado para usar o Codex exclusivamente via serviço cloud (Cloud Run).

## 🎯 Diferenças do CLI Original

| Recurso | codex (local) | codex-cloud |
|---------|---------------|-------------|
| Execução | Local | Cloud Run |
| Autenticação | API Keys locais | gcloud auth |
| Sandboxing | Local com landlock | Cloud com isolamento |
| Recursos | Limitado à máquina | 2GB RAM, escalável |
| Persistência | Local | Google Cloud Storage |
| Custo | Apenas API LLM | API LLM + Cloud Run |

## 📦 Instalação

### Pré-requisitos

```bash
# 1. Instalar gcloud CLI
# macOS
brew install --cask google-cloud-sdk

# Linux
curl https://sdk.cloud.google.com | bash

# 2. Autenticar com a conta Nexcode
gcloud auth login adm@nexcode.live

# 3. Configurar projeto
gcloud config set project elaihub-prod
```

### Compilar o CLI

```bash
cd /Users/williamduarte/NCMproduto/codex/codex-rs

# Compilar apenas o cloud-cli
cargo build --release -p codex-cloud-cli

# O binário estará em:
# target/release/codex-cloud
```

### Instalar Globalmente

```bash
# Copiar para /usr/local/bin
sudo cp target/release/codex-cloud /usr/local/bin/

# Verificar instalação
codex-cloud --version
```

## 🚀 Uso Básico

### Modo Exec (não-interativo)

```bash
# Executar comando simples
codex-cloud exec "What is 2+2?"

# Com modelo específico
codex-cloud exec "Create a Python hello world" --model gpt-4o

# Com timeout personalizado
codex-cloud exec "Complex task" --timeout 120

# Salvar resposta em arquivo
codex-cloud exec "Explain quantum computing" > explanation.txt
```

### Modo Interativo

```bash
# Iniciar sessão interativa
codex-cloud

# Ou explicitamente
codex-cloud interactive
```

### Comandos de Autenticação

```bash
# Verificar autenticação
codex-cloud login --status

# Fazer login novamente
gcloud auth login adm@nexcode.live

# Renovar token (se expirou)
gcloud auth application-default login
```

## 📋 Comandos Disponíveis

```bash
# Executar prompt
codex-cloud exec <prompt>

# Modo interativo (TUI)
codex-cloud interactive
codex-cloud  # atalho

# Modo proto (stdin/stdout)
codex-cloud proto

# Gerenciar sessões cloud
codex-cloud sessions list
codex-cloud sessions show <session-id>
codex-cloud sessions download <session-id>

# Upload de arquivos para contexto
codex-cloud upload <file>

# Configuração
codex-cloud config set model gpt-4o-mini
codex-cloud config set timeout 60
codex-cloud config show

# Ajuda
codex-cloud --help
codex-cloud exec --help
```

## ⚙️ Configuração

### Arquivo de Configuração

O CLI lê configurações de `~/.config/codex-cloud/config.toml`:

```toml
# ~/.config/codex-cloud/config.toml

[cloud]
# URL do serviço (padrão: production)
url = "https://codex-wrapper-467992722695.us-central1.run.app"

# Modelo padrão
model = "gpt-4o-mini"

# Timeout padrão em segundos
timeout = 60

# Cache de token em memória (segundos)
token_cache_duration = 3600

[auth]
# Conta gcloud a usar
account = "adm@nexcode.live"

# Projeto GCP
project = "elaihub-prod"
```

### Variáveis de Ambiente

```bash
# Sobrescrever URL do serviço
export CODEX_CLOUD_URL="https://codex-wrapper-467992722695.us-central1.run.app"

# Modelo padrão
export CODEX_CLOUD_MODEL="gpt-4o"

# Timeout padrão
export CODEX_CLOUD_TIMEOUT="120"

# Nível de log
export RUST_LOG="info"
export RUST_LOG="debug"  # Para debugging
```

## 🔐 Autenticação

### Fluxo de Autenticação

1. CLI chama `gcloud auth print-identity-token`
2. Token é armazenado em cache (1 hora)
3. Token é enviado em todas as requisições via header `Authorization: Bearer <token>`
4. Se token expirar, CLI solicita novo automaticamente

### Troubleshooting Autenticação

**Erro: "Falha ao obter token do gcloud"**

```bash
# Fazer login novamente
gcloud auth login adm@nexcode.live

# Verificar conta ativa
gcloud auth list

# Deve mostrar:
# * adm@nexcode.live
```

**Erro: "401 Unauthorized"**

```bash
# Token pode ter expirado
# Forçar renovação
gcloud auth application-default login

# Ou limpar cache e tentar novamente
rm -rf ~/.cache/codex-cloud/
```

**Erro: "403 Forbidden"**

```bash
# Verificar se tem permissão no domínio
# O serviço está configurado para: domain:nexcode.live
# Certifique-se de estar usando conta @nexcode.live
```

## 📊 Exemplos Práticos

### Exemplo 1: Criar e Executar Script

```bash
codex-cloud exec "Create a Python script that fetches weather data from wttr.in for Fortaleza and execute it"
```

### Exemplo 2: Análise de Código

```bash
codex-cloud exec "Analyze the Rust code in src/main.rs and suggest improvements"
```

### Exemplo 3: Geração de Documentação

```bash
codex-cloud exec "Generate API documentation from the comments in api.rs" > docs/api.md
```

### Exemplo 4: Pipeline com Arquivos

```bash
# Upload contexto
codex-cloud upload myfile.txt

# Processar
codex-cloud exec "Analyze myfile.txt and create a summary"

# Baixar resultado
codex-cloud sessions download $(codex-cloud sessions list | head -1)
```

## 🔄 Comparação com CLI Local

### Quando Usar `codex` (local)

✅ Desenvolvimento local rápido
✅ Acesso a arquivos locais sensíveis
✅ Sem custo de cloud
✅ Funciona offline (com API keys)

### Quando Usar `codex-cloud`

✅ Tarefas pesadas/longas
✅ Escalabilidade necessária
✅ Compartilhamento de sessões
✅ Auditoria centralizada
✅ Isolamento de segurança

## 🐛 Debug e Logs

### Ativar Logs Detalhados

```bash
# Logs de debug
RUST_LOG=debug codex-cloud exec "test"

# Logs de trace (muito verboso)
RUST_LOG=trace codex-cloud exec "test"

# Logs apenas do cloud-cli
RUST_LOG=codex_cloud_cli=debug codex-cloud exec "test"
```

### Ver Logs do Serviço Cloud

```bash
# Via gcloud
gcloud run services logs read codex-wrapper --region=us-central1 --limit=50

# Logs em tempo real
gcloud run services logs tail codex-wrapper --region=us-central1

# Filtrar por erro
gcloud run services logs read codex-wrapper --region=us-central1 | grep ERROR
```

## 📈 Performance

### Benchmarks Típicos

| Operação | Tempo | Observação |
|----------|-------|------------|
| Autenticação (primeira vez) | ~1s | Cache por 1h |
| Autenticação (cached) | <10ms | Em memória |
| Pergunta simples | 2-5s | Depende do modelo |
| Criar + executar script | 5-15s | Inclui execução |
| Upload arquivo (1MB) | 1-2s | Via GCS |

### Otimizações

```bash
# Usar modelo mais rápido
codex-cloud exec "quick task" --model gpt-4o-mini

# Timeout menor para tarefas rápidas
codex-cloud exec "2+2" --timeout 10

# Reusar sessão
codex-cloud exec "continue from last session" --session <session-id>
```

## 🔒 Segurança

### O que é Enviado para o Cloud

- ✅ Prompt do usuário
- ✅ Configurações de execução
- ✅ Arquivos enviados via upload
- ❌ **NÃO**: Credenciais locais
- ❌ **NÃO**: Código-fonte completo (apenas contexto necessário)

### Boas Práticas

1. **Nunca envie secrets em prompts**
   ```bash
   # ❌ RUIM
   codex-cloud exec "Use API key sk-123456..."

   # ✅ BOM
   codex-cloud exec "Use API key from environment variable"
   ```

2. **Use sessões privadas para dados sensíveis**
   ```bash
   codex-cloud exec "..." --private
   ```

3. **Revise logs periodicamente**
   ```bash
   codex-cloud sessions list | grep "sensitive"
   ```

## 🆘 Suporte

### Problemas Comuns

1. **"gcloud: command not found"**
   - Instale o gcloud CLI: https://cloud.google.com/sdk/docs/install

2. **"Token expirado"**
   - Execute: `gcloud auth login adm@nexcode.live`

3. **"Service unavailable"**
   - Verifique status: `gcloud run services describe codex-wrapper --region=us-central1`

4. **"Timeout"**
   - Aumente com: `--timeout 120`

### Reportar Issues

- Email: adm@nexcode.live
- Logs: Anexe output de `RUST_LOG=debug codex-cloud ...`

## 📚 Links Úteis

- [Documentação do Serviço Cloud](/Users/williamduarte/NCMproduto/codex/codex-rs/wrapper-cloud-run/README.md)
- [Guia Completo de Uso](/Users/williamduarte/NCMproduto/codex/codex-rs/wrapper-cloud-run/GUIA_COMPLETO_USO.md)
- [Codex CLI Original](/Users/williamduarte/NCMproduto/codex/codex-rs/cli/README.md)

---

**Versão**: 1.0.0
**Última Atualização**: 2025-10-03
**Mantido por**: Nexcode Team
