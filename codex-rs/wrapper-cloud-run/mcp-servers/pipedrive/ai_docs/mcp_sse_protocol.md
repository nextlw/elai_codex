# MCP SSE Protocol - Guia para LLMs

Este documento descreve o protocolo Server-Sent Events (SSE) usado pelo servidor MCP Pipedrive e como clientes devem interagir com ele.

## ⚠️ IMPORTANTE: Ordem de Operações

O protocolo SSE **REQUER** que as operações sejam realizadas na seguinte ordem:

1. **PRIMEIRO**: Estabelecer conexão SSE via GET `/sse`
2. **DEPOIS**: Enviar mensagens JSON-RPC via POST `/messages/?session_id=XXX`

**NUNCA envie mensagens para `/messages/` sem antes obter um `session_id` válido do endpoint `/sse`.**

---

## Fluxo Completo de Comunicação

### Passo 1: Estabelecer Conexão SSE

**Endpoint:** `GET /sse`

**Descrição:** Este endpoint inicia uma conexão Server-Sent Events e retorna um `session_id` único que deve ser usado em todas as mensagens subsequentes.

**Exemplo com curl:**
```bash
curl -N http://localhost:8152/sse
```

**Resposta SSE:**
```
event: endpoint
data: /messages/?session_id=abc123def456789...
```

**O que extrair da resposta:**
- O `session_id` está na query string do valor `data`
- No exemplo acima: `session_id=abc123def456789`

### Passo 2: Enviar Mensagens JSON-RPC

**Endpoint:** `POST /messages/?session_id={session_id}`

**Headers:**
```
Content-Type: application/json
```

**Body:** Mensagem JSON-RPC válida

**Exemplo com curl:**
```bash
curl -X POST "http://localhost:8152/messages/?session_id=abc123def456789" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "id": 1
  }'
```

**Resposta de Sucesso (202 Accepted):**
```
Accepted
```

---

## Exemplo Completo em Python

```python
import requests
import json
from sseclient import SSEClient

# Passo 1: Conectar ao SSE e obter session_id
print("Conectando ao SSE...")
sse_response = requests.get('http://localhost:8152/sse', stream=True)
client = SSEClient(sse_response)

session_id = None
for event in client.events():
    if event.event == 'endpoint':
        # Extrair session_id do formato: /messages/?session_id=XXX
        endpoint_data = event.data
        session_id = endpoint_data.split('session_id=')[1]
        print(f"Session ID obtido: {session_id}")
        break

if not session_id:
    raise Exception("Falha ao obter session_id")

# Passo 2: Usar session_id para enviar mensagens
message_url = f'http://localhost:8152/messages/?session_id={session_id}'

# Listar ferramentas disponíveis
response = requests.post(message_url, json={
    "jsonrpc": "2.0",
    "method": "tools/list",
    "id": 1
})
print(f"Status: {response.status_code}")
print(f"Response: {response.text}")

# Chamar uma ferramenta específica
response = requests.post(message_url, json={
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
        "name": "list_deals_from_pipedrive",
        "arguments": {}
    },
    "id": 2
})
print(f"Deals: {response.text}")
```

---

## Exemplo Completo em JavaScript/TypeScript

```typescript
import fetch from 'node-fetch';
import { EventSource } from 'eventsource';

async function connectMCP() {
  // Passo 1: Conectar ao SSE
  const sse = new EventSource('http://localhost:8152/sse');

  const sessionId = await new Promise<string>((resolve, reject) => {
    sse.addEventListener('endpoint', (event) => {
      const data = event.data;
      const sessionId = data.split('session_id=')[1];
      console.log('Session ID obtido:', sessionId);
      resolve(sessionId);
    });

    sse.addEventListener('error', (error) => {
      reject(error);
    });
  });

  // Passo 2: Enviar mensagens JSON-RPC
  const messageUrl = `http://localhost:8152/messages/?session_id=${sessionId}`;

  // Listar ferramentas
  const response = await fetch(messageUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      method: 'tools/list',
      id: 1
    })
  });

  console.log('Status:', response.status);
  console.log('Response:', await response.text());
}

connectMCP();
```

---

## Endpoints Disponíveis

### Protocolo MCP SSE

| Endpoint | Método | Descrição | Requer session_id? |
|----------|--------|-----------|-------------------|
| `/sse` | GET | Estabelece conexão SSE e retorna `session_id` | ❌ Não |
| `/messages/?session_id=XXX` | POST | Recebe mensagens JSON-RPC do cliente | ✅ Sim |

### Endpoints HTTP Customizados (Independentes)

| Endpoint | Método | Descrição |
|----------|--------|-----------|
| `/sessions` | POST | Cria sessão HTTP com metadados |
| `/sessions/{id}` | GET | Obtém dados de sessão HTTP |
| `/sessions/{id}` | DELETE | Deleta sessão HTTP |
| `/health` | GET | Health check do servidor |

**Nota:** As sessões HTTP (`/sessions`) são **independentes** das sessões SSE. Não use `session_id` de `/sessions` no endpoint `/messages/`.

---

## Métodos JSON-RPC Disponíveis

### Gerenciamento de Ferramentas

- `tools/list` - Lista todas as ferramentas disponíveis
- `tools/call` - Chama uma ferramenta específica

**Exemplo de chamada de ferramenta:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "create_deal_in_pipedrive",
    "arguments": {
      "title": "Novo Deal",
      "value": "5000",
      "currency": "USD"
    }
  },
  "id": 3
}
```

---

## Erros Comuns

### ❌ Erro: "session_id is required" (400)

**Causa:** Tentou enviar POST para `/messages/` sem `session_id` na query string.

**Solução:** Certifique-se de obter o `session_id` do endpoint `/sse` primeiro.

### ❌ Erro: "Invalid session ID" (400)

**Causa:** O `session_id` fornecido não é um UUID válido.

**Solução:** Use exatamente o `session_id` retornado pelo endpoint `/sse`, sem modificações.

### ❌ Erro: "Could not find session" (404)

**Causa:** O `session_id` expirou ou nunca existiu.

**Solução:** Reconecte ao endpoint `/sse` para obter um novo `session_id`.

### ❌ Erro: "Could not parse message" (400)

**Causa:** O body da requisição não é JSON-RPC válido.

**Solução:** Certifique-se de que o body contém `jsonrpc`, `method`, e `id`.

---

## Configuração do Servidor

O servidor pode ser configurado via variáveis de ambiente:

```bash
HOST=127.0.0.1      # Host (127.0.0.1 local, 0.0.0.0 containers)
PORT=8152           # Porta do servidor
TRANSPORT=sse       # Tipo de transporte (sse ou stdio)
```

---

## Resumo para LLMs

**Para usar o servidor MCP Pipedrive via SSE:**

1. ✅ Faça `GET /sse` e aguarde o evento `endpoint`
2. ✅ Extraia o `session_id` da mensagem recebida
3. ✅ Use esse `session_id` em todos os `POST /messages/?session_id=XXX`
4. ✅ Envie mensagens JSON-RPC no body do POST
5. ✅ Mantenha a conexão SSE aberta para receber respostas

**Não faça:**
- ❌ POST para `/messages/` sem obter `session_id` primeiro
- ❌ Usar `session_id` de `/sessions` (são sistemas diferentes)
- ❌ Modificar ou inventar `session_id`
- ❌ Fechar a conexão SSE prematuramente

---

**Última atualização:** 07/10/2025
