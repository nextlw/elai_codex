
# 📚 DOCUMENTAÇÃO COMPLETA DA API GATEWAY CODEX

## 🎯 **VISÃO GERAL**

Gateway HTTP/WebSocket que serve como fundação para todos os serviços Codex, oferecendo 4 endpoints principais com configurações avançadas.

---

## 🚀 **INICIANDO O GATEWAY**

### **Comando de Execução**

```bash
cd codex-rs
cargo run --package codex-gateway
```

### **Configuração via Environment Variables**

```bash
# Configurações básicas
export GATEWAY_HOST=0.0.0.0
export GATEWAY_PORT=8080

# Timeouts
export GATEWAY_REQUEST_TIMEOUT_SECS=30
export GATEWAY_KEEP_ALIVE_TIMEOUT_SECS=60

# Body limits globais
export GATEWAY_BODY_LIMIT_DEFAULT=2097152          # 2MB
export GATEWAY_BODY_LIMITS_ENABLED=true

# Body limits específicos por endpoint
export GATEWAY_BODY_LIMIT_HEALTH=1024              # 1KB
export GATEWAY_BODY_LIMIT_JSONRPC=1048576           # 1MB
export GATEWAY_BODY_LIMIT_WEBSOCKET=1048576         # 1MB
export GATEWAY_BODY_LIMIT_WEBHOOK=10485760          # 10MB

# WebSocket configurações
export GATEWAY_WEBSOCKET_MAX_MESSAGE_SIZE=67108864  # 64MB
export GATEWAY_WEBSOCKET_MAX_FRAME_SIZE=16777216    # 16MB
export GATEWAY_WEBSOCKET_MAX_CONNECTIONS=5000
export GATEWAY_WEBSOCKET_COMPRESSION_ENABLED=true
export GATEWAY_WEBSOCKET_PING_INTERVAL_SECS=30
export GATEWAY_WEBSOCKET_TIMEOUT_SECS=300
```

---

## 📋 **ENDPOINTS DISPONÍVEIS**

### **1. 🏥 HEALTH CHECK ENDPOINT**

#### **GET /health**

Endpoint para verificação de saúde do sistema.

**Request:**

```bash
curl -X GET http://localhost:8080/health
```

**Response Success (200 OK):**

```json
{
  "status": "healthy"
}
```

**Headers:**

- `Content-Type: application/json`
- `Content-Length: 21`

**Limites:**

- Body size: 1KB máximo
- Timeout: 30s (configurável)

---

### **2. 🔌 JSON-RPC ENDPOINT**

#### **POST /jsonrpc**

Endpoint principal para comandos Codex via protocolo JSON-RPC 2.0 com **integração real ao Codex Core**.

---

#### **Método: conversation.prompt**

Executa um prompt de IA usando o Codex real.

**Request:**

```bash
curl -X POST http://localhost:8080/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "conversation.prompt",
    "params": {
      "prompt": "Write a Rust function that adds two numbers",
      "session_id": "my-session-123"
    },
    "id": 1
  }'
```

**Request Schema:**

```json
{
  "jsonrpc": "2.0",
  "method": "conversation.prompt",
  "params": {
    "prompt": "string",           // Required: Prompt text
    "session_id": "string"        // Optional: Session ID for continuity
  },
  "id": 1
}
```

**Response Success (200 OK):**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "type": "ai_response",
    "conversation_id": "conv_abc123",
    "content": "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}",
    "model": "claude-3-sonnet",
    "timestamp": "2024-11-13T14:20:00.000Z",
    "events": [
      {
        "TaskStarted": { /* ... */ }
      },
      {
        "AgentMessage": { "message": "..." }
      },
      {
        "TaskComplete": { /* ... */ }
      }
    ]
  },
  "id": 1
}
```

---

#### **Método: conversation.status**

Obtém o status de uma sessão/conversação ativa.

**Request:**

```bash
curl -X POST http://localhost:8080/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "conversation.status",
    "params": {
      "session_id": "my-session-123"
    },
    "id": 2
  }'
```

**Response Success (200 OK):**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "conversation_id": "conv_abc123",
    "metadata": {
      "model": "claude-3-sonnet",
      "created_at": "2024-11-13T14:00:00.000Z"
    }
  },
  "id": 2
}
```

**Response Not Found (200 OK):**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "not_found",
    "session_id": "my-session-123"
  },
  "id": 2
}
```

---

#### **Método: conversation.cancel**

Cancela uma sessão/conversação ativa.

**Request:**

```bash
curl -X POST http://localhost:8080/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "conversation.cancel",
    "params": {
      "session_id": "my-session-123"
    },
    "id": 3
  }'
```

**Response Success (200 OK):**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "cancelled": true,
    "session_id": "my-session-123",
    "conversation_id": "conv_abc123"
  },
  "id": 3
}
```

---

**Response Error Examples:**

**Missing Parameters (400 Bad Request):**

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Missing required parameter 'prompt'"
  },
  "id": 1
}
```

**Method Not Found (400 Bad Request):**

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Method 'unknown_method' not found",
    "data": {
      "available_methods": [
        "conversation.prompt",
        "conversation.status",
        "conversation.cancel"
      ]
    }
  },
  "id": 1
}
```

**Invalid JSON-RPC Version:**

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32600,
    "message": "Invalid Request: JSON-RPC version must be '2.0'"
  },
  "id": null
}
```

---

**Métodos Suportados:**

- ✅ `conversation.prompt` - Executar prompts de IA (integração real com Codex Core)
- ✅ `conversation.status` - Obter status de uma conversação
- ✅ `conversation.cancel` - Cancelar conversação ativa

**Limites:**

- Body size: 1MB máximo
- Timeout: 30s (configurável)

---

### **3. 🔗 WEBSOCKET ENDPOINT**

#### **GET /ws (WebSocket Upgrade)**

Endpoint para conexões WebSocket persistentes.

**WebSocket Handshake:**

```bash
# Usando curl (apenas para teste de upgrade)
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==" \
  http://localhost:8080/ws
```

**Usando wscat:**

```bash
# Instalar wscat: npm install -g wscat
wscat -c ws://localhost:8080/ws
```

**Response Success (101 Switching Protocols):**

```http
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: HSmrc0sMlYUkAGmm5OPpG2HaGWk=
```

**Mensagens WebSocket:**

```javascript
// Conectar
const ws = new WebSocket('ws://localhost:8080/ws');

// Enviar mensagem
ws.send(JSON.stringify({
  type: "command",
  payload: "create hello world script"
}));

// Receber mensagem
ws.onmessage = function(event) {
  console.log('Received:', event.data);
};
```

**Configurações:**

- Max connections: 5,000 (configurável)
- Max message size: 64MB (configurável)
- Max frame size: 16MB (configurável)
- Ping interval: 30s (configurável)
- Connection timeout: 300s (configurável)
- Compression: Habilitado (configurável)

---

### **4. 🎣 WEBHOOK ENDPOINT**

#### **POST /webhook**

Endpoint para receber webhooks de serviços externos (GitHub, etc.).

**Request:**

```bash
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -H "X-GitHub-Event: push" \
  -d '{
    "ref": "refs/heads/main",
    "repository": {
      "name": "codex-project",
      "full_name": "user/codex-project"
    },
    "commits": [
      {
        "id": "abc123",
        "message": "Add new feature",
        "author": {
          "name": "Developer",
          "email": "dev@example.com"
        }
      }
    ]
  }'
```

**Response Success (202 Accepted):**

```json
{
  "status": "accepted",
  "message": "Webhook received and queued for processing"
}
```

**Headers Suportados:**

- `X-GitHub-Event` - Tipo de evento GitHub
- `X-GitHub-Delivery` - ID de entrega GitHub
- `X-Hub-Signature` - Assinatura de segurança
- `User-Agent` - Cliente que enviou

**Limites:**

- Body size: 10MB máximo (para repos grandes)
- Timeout: 30s (configurável)

---

## ⚠️ **RESPOSTAS DE ERRO**

### **413 Payload Too Large**

```json
{
  "error": "Request body too large for endpoint '/jsonrpc' (max 1048576 bytes allowed)",
  "status": 413,
  "details": {
    "max_size": 1048576,
    "endpoint": "/jsonrpc"
  }
}
```

### **408 Request Timeout**

```json
{
  "error": "Request timed out",
  "status": 408
}
```

### **400 Bad Request**

```json
{
  "error": "Invalid JSON format",
  "status": 400
}
```

### **500 Internal Server Error**

```json
{
  "error": "Internal server error",
  "status": 500
}
```

---

## 🔧 **CONFIGURAÇÕES AVANÇADAS**

### **Body Size Limits por Endpoint**

| Endpoint     | Limite Padrão | Environment Variable             | Justificativa               |
| ------------ | -------------- | -------------------------------- | --------------------------- |
| `/health`  | 1KB            | `GATEWAY_BODY_LIMIT_HEALTH`    | Health checks são mínimos |
| `/jsonrpc` | 1MB            | `GATEWAY_BODY_LIMIT_JSONRPC`   | Comandos CLI complexos      |
| `/ws`      | 1MB            | `GATEWAY_BODY_LIMIT_WEBSOCKET` | Upgrade + mensagens         |
| `/webhook` | 10MB           | `GATEWAY_BODY_LIMIT_WEBHOOK`   | GitHub diffs grandes        |

### **Timeouts Configuráveis**

```bash
GATEWAY_REQUEST_TIMEOUT_SECS=30        # Timeout por request
GATEWAY_KEEP_ALIVE_TIMEOUT_SECS=60     # Keep-alive TCP
GATEWAY_WEBSOCKET_PING_INTERVAL_SECS=30 # Ping WebSocket
GATEWAY_WEBSOCKET_TIMEOUT_SECS=300      # Timeout WebSocket
```

---

## 📊 **LOGS E OBSERVABILIDADE**

### **Log Format (JSON Structured)**

```json
{
  "timestamp": "2024-11-11T21:54:00.000Z",
  "level": "INFO",
  "target": "codex_gateway::router",
  "message": "Request processed",
  "fields": {
    "method": "POST",
    "uri": "/jsonrpc",
    "status": 200,
    "latency_ms": 15,
    "body_size": 156
  }
}
```

### **Métricas Tracked**

- Request count por endpoint
- Latência por endpoint
- Body size violations
- WebSocket connections ativas
- Error rates por tipo

---

## 🚦 **MIDDLEWARE STACK**

### **Middleware Aplicado**

1. **Tracing Layer** - Logging estruturado
2. **CORS Layer** - Cross-origin requests
3. **Timeout Layer** - Request timeouts
4. **Body Limit Layer** - Size restrictions (específico por endpoint)

### **Headers CORS**

```http
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization
Access-Control-Max-Age: 3600
```

---

## 🧪 **TESTES E VALIDAÇÃO**

### **Script de Teste Completo**

```bash
#!/bin/bash
# Teste todos os endpoints com integração real ao Codex

BASE_URL="http://localhost:8080"

echo "1. Testando Health Check..."
curl -s "$BASE_URL/health" | jq

echo -e "\n2. Testando JSON-RPC - conversation.prompt..."
curl -s -X POST "$BASE_URL/jsonrpc" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "conversation.prompt",
    "params": {
      "prompt": "Write a Rust function that adds two numbers",
      "session_id": "test-session-001"
    },
    "id": 1
  }' | jq

echo -e "\n3. Testando JSON-RPC - conversation.status..."
curl -s -X POST "$BASE_URL/jsonrpc" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "conversation.status",
    "params": {
      "session_id": "test-session-001"
    },
    "id": 2
  }' | jq

echo -e "\n4. Testando JSON-RPC - método inválido..."
curl -s -X POST "$BASE_URL/jsonrpc" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "invalid_method",
    "id": 3
  }' | jq

echo -e "\n5. Testando WebSocket Upgrade..."
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: dGVzdA==" \
  "$BASE_URL/ws" | head -10

echo -e "\n6. Testando Webhook..."
curl -s -X POST "$BASE_URL/webhook" \
  -H "Content-Type: application/json" \
  -d '{"event": "test", "data": "webhook test"}' | jq

echo -e "\n7. Testando Body Size Limit..."
curl -s -X POST "$BASE_URL/jsonrpc" \
  -H "Content-Type: application/json" \
  -d "$(printf '{"jsonrpc": "2.0", "method": "conversation.prompt", "params": {"prompt": "%*s"}, "id": 1}' 1048577 "")" | jq
```

---

## 📋 **RESUMO DOS COMANDOS**

### **Inicialização**

```bash
cargo run --package codex-gateway
```

### **Endpoints Básicos**

```bash
# Health Check
curl http://localhost:8080/health

# JSON-RPC - Executar prompt de IA
curl -X POST http://localhost:8080/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "conversation.prompt",
    "params": {
      "prompt": "Write a hello world in Python"
    },
    "id": 1
  }'

# JSON-RPC - Verificar status
curl -X POST http://localhost:8080/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "conversation.status",
    "params": {
      "session_id": "my-session"
    },
    "id": 2
  }'

# JSON-RPC - Cancelar conversação
curl -X POST http://localhost:8080/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "conversation.cancel",
    "params": {
      "session_id": "my-session"
    },
    "id": 3
  }'

# WebSocket
wscat -c ws://localhost:8080/ws

# Webhook
curl -X POST http://localhost:8080/webhook -H "Content-Type: application/json" -d '{"event": "test"}'
```

### **Configuração**

```bash
# Configurar porta
export GATEWAY_PORT=3000

# Configurar limites
export GATEWAY_BODY_LIMIT_JSONRPC=2097152

# Configurar WebSocket
export GATEWAY_WEBSOCKET_MAX_CONNECTIONS=10000
```

---

## 🤖 **INTEGRAÇÃO REAL COM CODEX CORE**

O Gateway agora possui **integração nativa e completa** com o Codex Core:

### **CodexService - Ponte Real para IA**

```rust
// Inicialização automática do Codex Core
let service = CodexService::new().await?;

// Execução de prompts reais
let response = service.execute_prompt("Write a function", None).await?;
```

### **Estrutura da Resposta Real**

```json
{
  "type": "ai_response",
  "conversation_id": "conv_real_id",
  "content": "fn add(a: i32, b: i32) -> i32 { a + b }",
  "model": "claude-3-sonnet",
  "timestamp": "2024-11-13T14:20:00.000Z",
  "events": [
    { "TaskStarted": { /* evento real do Codex */ } },
    { "AgentMessage": { "message": "função implementada" } },
    { "TaskComplete": { /* evento de conclusão */ } }
  ]
}
```

### **Gerenciamento de Conversações**

- ✅ **ConversationManager** - Gerenciamento real de conversas via Codex Core
- ✅ **SessionSource::Exec** - Modo de execução nativo
- ✅ **Streaming de Eventos** - Todos os eventos do agente são capturados
- ✅ **Session Continuity** - Múltiplos turnos na mesma sessão

### **Fluxo de Execução Real**

```
User → JSON-RPC → CodexService → ConversationManager → Codex Core
                                                           ↓
User ← JSON-RPC ← CodexService ← Event Stream ← Agent Response
```

---

## 🧪 **TESTES DE INTEGRAÇÃO**

### **Teste Automatizado**

```bash
# Executar teste de integração real
cargo test --package codex-gateway --test execute_prompt_test

# Resultado esperado:
# ✅ test_execute_prompt_real ... ok
# ✅ test_execute_prompt_with_session ... ok
```

### **Validação da Resposta**

O teste valida:
- ✅ Estrutura JSON completa
- ✅ Campo `type: "ai_response"`
- ✅ `conversation_id` válido
- ✅ `content` não vazio
- ✅ Array de `events` do Codex
- ✅ Presença de indicadores Rust no conteúdo

---

## ✅ **STATUS**

O Gateway Codex oferece uma **API completa e robusta** com:

### **Funcionalidades Core**
- ✅ 4 endpoints funcionais
- ✅ **Integração REAL com Codex Core** (não placeholder!)
- ✅ JSON-RPC com 3 métodos funcionais
- ✅ Streaming de eventos do agente
- ✅ Gerenciamento de sessões/conversações

### **Infraestrutura**
- ✅ Configuração flexível via environment vars
- ✅ Body limits específicos por endpoint
- ✅ WebSocket support completo
- ✅ Error handling detalhado
- ✅ Observabilidade com logs estruturados
- ✅ Middleware stack profissional

### **Qualidade**
- ✅ Testes de integração com Codex real
- ✅ Compilação sem erros
- ✅ Propagação correta de erros (sem panics)
- ✅ Production-ready

**Status:** 🎉 **PRONTO PARA USO EM PRODUÇÃO**

- Integração real com Codex Core ✅
- API JSON-RPC funcional ✅
- Testes automatizados passando ✅
- Documentação completa ✅

**Próximas fases:** Worker pools especializados e escalabilidade cloud-native.
