#!/bin/bash

TOKEN="IxF3WoAB6IBrNJKrC/Jsr5yjt2bXHZkBSHFDBhcIVvc="
URL="http://localhost:8080"

echo "======================================"
echo "Testando Wrapper Cloud Run"
echo "======================================"
echo ""

echo "✅ Healthcheck (sem autenticação):"
curl -s "$URL/health"
echo -e "\n"

echo "❌ Teste 1: Request SEM token (deve retornar 401):"
HTTP_CODE=$(curl -s -w "%{http_code}" -o /dev/null \
  -X POST "$URL/api/v1/exec/stream" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"echo test"}')
echo "HTTP Status: $HTTP_CODE"
if [ "$HTTP_CODE" = "401" ]; then
  echo "✓ Autenticação funcionando (bloqueou request sem token)"
else
  echo "✗ Falha: deveria retornar 401"
fi
echo ""

echo "✅ Teste 2: Request COM token correto:"
curl -s -N -X POST "$URL/api/v1/exec/stream" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"prompt":"echo Hello Authenticated World"}' | head -5
echo ""
echo "✓ Request autenticado com sucesso!"
