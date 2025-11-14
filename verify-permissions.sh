#!/bin/bash
# Verificação de permissões do Cloud Build

PROJECT_ID="elaihub-prod"
CLOUD_BUILD_SA="467992722695@cloudbuild.gserviceaccount.com"
COMPUTE_SA="467992722695-compute@developer.gserviceaccount.com"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 VERIFICAÇÃO DE PERMISSÕES"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 1. Verificar serviceAccountUser
echo "1️⃣  Verificando permissão 'actAs' (serviceAccountUser)..."
ACTAS=$(gcloud iam service-accounts get-iam-policy $COMPUTE_SA \
  --format="value(bindings.members)" \
  --filter="bindings.role:roles/iam.serviceAccountUser" \
  --flatten="bindings[].members" 2>&1 | grep -c "$CLOUD_BUILD_SA" || echo "0")

if [ "$ACTAS" -gt 0 ]; then
  echo "   ✅ Cloud Build PODE atuar como Compute SA"
else
  echo "   ❌ Cloud Build NÃO PODE atuar como Compute SA"
  echo "   Execute: ./fix-cloud-build-permissions.sh"
fi
echo ""

# 2. Verificar Cloud Run Admin
echo "2️⃣  Verificando permissão 'run.admin'..."
RUN_ADMIN=$(gcloud projects get-iam-policy $PROJECT_ID \
  --flatten="bindings[].members" \
  --filter="bindings.members:$CLOUD_BUILD_SA AND bindings.role:roles/run.admin" \
  --format="value(bindings.role)" 2>&1 | wc -l)

if [ "$RUN_ADMIN" -gt 0 ]; then
  echo "   ✅ Cloud Build TEM permissão run.admin"
else
  echo "   ❌ Cloud Build NÃO TEM permissão run.admin"
fi
echo ""

# 3. Verificar Secret Manager
echo "3️⃣  Verificando acesso ao Secret Manager..."
SECRET_ACCESS=$(gcloud projects get-iam-policy $PROJECT_ID \
  --flatten="bindings[].members" \
  --filter="bindings.members:$CLOUD_BUILD_SA AND bindings.role:roles/secretmanager.secretAccessor" \
  --format="value(bindings.role)" 2>&1 | wc -l)

if [ "$SECRET_ACCESS" -gt 0 ]; then
  echo "   ✅ Cloud Build PODE acessar secrets"
else
  echo "   ⚠️  Cloud Build NÃO PODE acessar secrets"
  echo "   (Necessário se usar --set-secrets no deploy)"
fi
echo ""

# 4. Verificar se secrets existem
echo "4️⃣  Verificando se secrets necessários existem..."
SECRETS=(
  "gateway-api-key"
  "anthropic-api-key"
  "openai-api-key"
  "pipedrive-api-token"
)

MISSING_SECRETS=0
for SECRET in "${SECRETS[@]}"; do
  if gcloud secrets describe $SECRET --format="value(name)" &>/dev/null; then
    echo "   ✅ $SECRET existe"
  else
    echo "   ⚠️  $SECRET NÃO EXISTE"
    MISSING_SECRETS=$((MISSING_SECRETS + 1))
  fi
done
echo ""

# 5. Verificar Storage Admin
echo "5️⃣  Verificando permissão 'storage.admin'..."
STORAGE_ADMIN=$(gcloud projects get-iam-policy $PROJECT_ID \
  --flatten="bindings[].members" \
  --filter="bindings.members:$CLOUD_BUILD_SA AND bindings.role:roles/storage.admin" \
  --format="value(bindings.role)" 2>&1 | wc -l)

if [ "$STORAGE_ADMIN" -gt 0 ]; then
  echo "   ✅ Cloud Build TEM permissão storage.admin"
else
  echo "   ⚠️  Cloud Build NÃO TEM permissão storage.admin"
fi
echo ""

# Resumo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 RESUMO"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

ALL_OK=true

if [ "$ACTAS" -eq 0 ]; then
  echo "❌ CRÍTICO: Falta permissão serviceAccountUser"
  ALL_OK=false
fi

if [ "$RUN_ADMIN" -eq 0 ]; then
  echo "❌ CRÍTICO: Falta permissão run.admin"
  ALL_OK=false
fi

if [ "$MISSING_SECRETS" -gt 0 ]; then
  echo "⚠️  AVISO: $MISSING_SECRETS secret(s) não encontrado(s)"
  echo "   Você pode:"
  echo "   1. Criar os secrets manualmente, ou"
  echo "   2. Remover --set-secrets do cloudbuild.yaml"
fi

if [ "$ALL_OK" = true ] && [ "$MISSING_SECRETS" -eq 0 ]; then
  echo "✅ TUDO OK! Pronto para deploy"
elif [ "$ALL_OK" = true ]; then
  echo "✅ Permissões OK (mas verifique secrets)"
else
  echo "❌ Corrija as permissões antes de fazer deploy"
fi

echo ""
