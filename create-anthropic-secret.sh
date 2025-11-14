#!/bin/bash
# Create Anthropic API Key secret

PROJECT_ID="elaihub-prod"
SECRET_NAME="anthropic-api-key"
COMPUTE_SA="467992722695-compute@developer.gserviceaccount.com"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔐 Criando Secret: anthropic-api-key"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Prompt para a API key (não vou usar a exposta)
echo "⚠️  IMPORTANTE: Use uma NOVA chave (revogue a anterior!)"
echo ""
read -sp "Digite a NOVA Anthropic API Key: " API_KEY
echo ""

if [ -z "$API_KEY" ]; then
  echo "❌ API Key não pode ser vazia"
  exit 1
fi

echo ""
echo "1️⃣  Criando secret..."

# Verificar se já existe
if gcloud secrets describe $SECRET_NAME --project=$PROJECT_ID &>/dev/null; then
  echo "   ℹ️  Secret já existe, adicionando nova versão..."
  echo -n "$API_KEY" | gcloud secrets versions add $SECRET_NAME \
    --project=$PROJECT_ID \
    --data-file=-
else
  echo "   📦 Criando novo secret..."
  echo -n "$API_KEY" | gcloud secrets create $SECRET_NAME \
    --project=$PROJECT_ID \
    --replication-policy="automatic" \
    --data-file=-
fi

echo "   ✅ Secret criado/atualizado"
echo ""

echo "2️⃣  Adicionando permissão para Cloud Run..."
gcloud secrets add-iam-policy-binding $SECRET_NAME \
  --project=$PROJECT_ID \
  --member="serviceAccount:$COMPUTE_SA" \
  --role="roles/secretmanager.secretAccessor" \
  --quiet

echo "   ✅ Permissão adicionada"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ SECRET CONFIGURADO!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🚨 PRÓXIMOS PASSOS:"
echo "   1. REVOGUE a chave antiga em: https://console.anthropic.com/settings/keys"
echo "   2. Clique em 'tentar novamente' no Cloud Build"
echo ""
