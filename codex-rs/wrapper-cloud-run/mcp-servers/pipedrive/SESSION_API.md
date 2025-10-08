# API de Sessões - MCP Pipedrive

Este documento descreve os endpoints HTTP customizados disponíveis no servidor MCP Pipedrive para gerenciamento de sessões.

## Visão Geral

O servidor MCP Pipedrive agora inclui endpoints HTTP para criar e gerenciar sessões de usuário. Essas sessões podem ser usadas para rastrear interações de usuários com o sistema.

## Endpoints

### 1. Criar Sessão

Cria uma nova sessão de usuário.

**Endpoint:** `POST /sessions`

**Body (opcional):**
```json
{
  "user_id": "user123",
  "metadata": {
    "key": "value"
  }
}
```

**Resposta de Sucesso (200):**
```json
{
  "success": true,
  "session_id": "abc123def456...",
  "created_at": "2025-01-06T12:00:00",
  "expires_at": "2025-01-07T12:00:00"
}
```

**Exemplo com curl:**
```bash
curl -X POST http://localhost:8152/sessions \
  -H "Content-Type: application/json" \
  -d '{"user_id": "user123", "metadata": {"source": "web"}}'
```

**Exemplo com httpx (Python):**
```python
import httpx

async with httpx.AsyncClient() as client:
    response = await client.post(
        "http://localhost:8152/sessions",
        json={"user_id": "user123", "metadata": {"source": "web"}}
    )
    result = response.json()
    session_id = result["session_id"]
```

---

### 2. Obter Sessão

Obtém informações de uma sessão existente.

**Endpoint:** `GET /sessions/{session_id}`

**Resposta de Sucesso (200):**
```json
{
  "success": true,
  "session": {
    "session_id": "abc123def456...",
    "created_at": "2025-01-06T12:00:00",
    "expires_at": "2025-01-07T12:00:00",
    "user_id": "user123",
    "metadata": {
      "key": "value"
    }
  }
}
```

**Resposta de Erro (404):**
```json
{
  "success": false,
  "error": "Sessão não encontrada ou expirada"
}
```

**Exemplo com curl:**
```bash
curl http://localhost:8152/sessions/abc123def456
```

---

### 3. Deletar Sessão

Deleta uma sessão existente.

**Endpoint:** `DELETE /sessions/{session_id}`

**Resposta de Sucesso (200):**
```json
{
  "success": true,
  "message": "Sessão deletada com sucesso"
}
```

**Resposta de Erro (404):**
```json
{
  "success": false,
  "error": "Sessão não encontrada"
}
```

**Exemplo com curl:**
```bash
curl -X DELETE http://localhost:8152/sessions/abc123def456
```

---

### 4. Health Check

Verifica o status do servidor e o número de sessões ativas.

**Endpoint:** `GET /health`

**Resposta (200):**
```json
{
  "status": "ok",
  "active_sessions": 5
}
```

**Exemplo com curl:**
```bash
curl http://localhost:8152/health
```

---

## Configuração

### Tempo de Vida das Sessões

Por padrão, as sessões expiram após 24 horas. Este valor está configurado no `SessionManager` e pode ser ajustado modificando o parâmetro `default_ttl_hours`.

### Armazenamento

Atualmente, as sessões são armazenadas em memória. Para ambientes de produção, considere implementar um backend persistente como:
- Redis
- Firestore
- PostgreSQL

## Integração com o Sistema

### No nexcode-api

O `nexcode-api` pode criar sessões chamando o endpoint POST /sessions:

```typescript
// No nexcode-api
const response = await fetch('http://localhost:8152/sessions', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    user_id: userId,
    metadata: { source: 'nexcode-api' }
  })
});

const { session_id } = await response.json();
// Armazenar session_id para uso posterior
```

### Cleanup Automático

O sistema automaticamente remove sessões expiradas quando são acessadas. Para limpeza proativa periódica, você pode adicionar um job agendado:

```python
import asyncio
from pipedrive.session_manager import get_session_manager

async def cleanup_task():
    while True:
        await asyncio.sleep(3600)  # A cada hora
        manager = get_session_manager()
        removed = manager.cleanup_expired_sessions()
        print(f"Removidas {removed} sessões expiradas")
```

## Teste

Use o script de teste incluído para verificar os endpoints:

```bash
uv run python test_sessions.py
```

## Segurança

⚠️ **Importante:**
- As sessões atualmente não têm autenticação adicional
- Qualquer um com o `session_id` pode acessar a sessão
- Em produção, adicione:
  - Autenticação JWT ou similar
  - Rate limiting
  - HTTPS obrigatório
  - Validação de origem (CORS)
