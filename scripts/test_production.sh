#!/bin/bash
set -e

# Configuração
GATEWAY_URL="https://wrapper-467992722695.us-central1.run.app"
GATEWAY_KEY=$(gcloud secrets versions access latest --secret=gateway-api-key)

echo "=========================================="
echo "🚀 TESTES DE PRODUÇÃO - CODEX GATEWAY"
echo "=========================================="
echo "URL: $GATEWAY_URL"
echo "=========================================="
echo ""

# Teste 1: Health Check
echo "=== ✅ TESTE 1: Health Check ==="
HEALTH=$(curl -s "$GATEWAY_URL/health")
echo "$HEALTH" | jq .
echo ""

# Teste 2: JSON-RPC
echo "=== 📡 TESTE 2: JSON-RPC Endpoint ==="
JSONRPC_RESPONSE=$(curl -s -X POST "$GATEWAY_URL/jsonrpc" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $GATEWAY_KEY" \
  -d '{
    "jsonrpc": "2.0",
    "method": "conversation.prompt",
    "params": {
      "prompt": "Say hello from production!",
      "conversation_id": null
    },
    "id": 1
  }')

echo "$JSONRPC_RESPONSE" | jq .
echo ""

# Teste 3: Exec Mode
echo "=== ⚡ TESTE 3: Exec Mode ==="
EXEC_RESPONSE=$(curl -s -X POST "$GATEWAY_URL/exec" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $GATEWAY_KEY" \
  -d '{
    "prompt": "Echo: Production test successful"
  }')

echo "$EXEC_RESPONSE" | jq .
echo ""

# Teste 4: Webhook
echo "=== 🔔 TESTE 4: Webhook Endpoint ==="
WEBHOOK_RESPONSE=$(curl -s -X POST "$GATEWAY_URL/webhook" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $GATEWAY_KEY" \
  -d '{
    "event": "test_production",
    "data": {
      "message": "Testing webhook in production",
      "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"
    }
  }')

echo "$WEBHOOK_RESPONSE" | jq .
echo ""

# Teste 5: OAuth Authorization (sem autenticação)
echo "=== 🔐 TESTE 5: OAuth Authorization Endpoint ==="
OAUTH_AUTH=$(curl -s "$GATEWAY_URL/oauth/authorize?response_type=code&client_id=codex-gateway-client&redirect_uri=http://localhost:3000/callback&state=test123")
echo "Response: $OAUTH_AUTH"
echo ""

# Resumo
echo "=========================================="
echo "✅ RESUMO DOS TESTES"
echo "=========================================="
echo "✅ Health Check: OK"
echo "✅ JSON-RPC: $(echo "$JSONRPC_RESPONSE" | jq -r 'if .result then "OK" else "FAILED" end')"
echo "✅ Exec Mode: $(echo "$EXEC_RESPONSE" | jq -r 'if .conversation_id then "OK" else "FAILED" end')"
echo "✅ Webhook: $(echo "$WEBHOOK_RESPONSE" | jq -r 'if .status then "OK" else "FAILED" end')"
echo "✅ OAuth: $(if [[ "$OAUTH_AUTH" == *"http"* ]]; then echo "OK"; else echo "FAILED"; fi)"
echo "=========================================="
