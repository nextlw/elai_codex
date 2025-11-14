#!/bin/bash
# Fix Cloud Build permissions to deploy to Cloud Run

set -e

PROJECT_ID="elaihub-prod"
PROJECT_NUMBER="467992722695"
CLOUD_BUILD_SA="${PROJECT_NUMBER}@cloudbuild.gserviceaccount.com"
COMPUTE_SA="${PROJECT_NUMBER}-compute@developer.gserviceaccount.com"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔧 Fixing Cloud Build → Cloud Run Permissions"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Cloud Build SA: $CLOUD_BUILD_SA"
echo "Compute SA: $COMPUTE_SA"
echo ""

# Cloud Build precisa atuar como a Compute Service Account
echo "1️⃣  Permitindo Cloud Build atuar como Compute SA..."
gcloud iam service-accounts add-iam-policy-binding \
  $COMPUTE_SA \
  --member="serviceAccount:$CLOUD_BUILD_SA" \
  --role="roles/iam.serviceAccountUser" \
  --condition=None

echo "   ✅ Permissão concedida"
echo ""

# Cloud Build também precisa de permissão de Cloud Run Admin
echo "2️⃣  Verificando permissões de Cloud Run Admin..."
gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:$CLOUD_BUILD_SA" \
  --role="roles/run.admin" \
  --condition=None

echo "   ✅ Cloud Run Admin confirmado"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ PERMISSÕES CORRIGIDAS!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Agora o Cloud Build pode:"
echo "  ✅ Fazer deploy no Cloud Run"
echo "  ✅ Usar a Compute Service Account"
echo ""
