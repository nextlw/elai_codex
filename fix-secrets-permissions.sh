#!/bin/bash
# Fix permissions for Cloud Run to access secrets

PROJECT_ID="elaihub-prod"
COMPUTE_SA="467992722695-compute@developer.gserviceaccount.com"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔐 Configurando Acesso aos Secrets"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Service Account: $COMPUTE_SA"
echo ""

SECRETS=(
  "gateway-api-key"
  "anthropic-api-key"
  "openai-api-key"
  "pipedrive-api-token"
)

echo "Adicionando permissão secretAccessor aos secrets..."
echo ""

for SECRET in "${SECRETS[@]}"; do
  echo "📦 $SECRET..."

  gcloud secrets add-iam-policy-binding $SECRET \
    --member="serviceAccount:$COMPUTE_SA" \
    --role="roles/secretmanager.secretAccessor" \
    --quiet 2>&1 | grep -v "Updated IAM policy" || true

  echo "   ✅ Permissão adicionada"
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ PERMISSÕES CONFIGURADAS!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "A service account $COMPUTE_SA agora pode:"
echo "  ✅ Acessar gateway-api-key"
echo "  ✅ Acessar anthropic-api-key"
echo "  ✅ Acessar openai-api-key"
echo "  ✅ Acessar pipedrive-api-token"
echo ""
