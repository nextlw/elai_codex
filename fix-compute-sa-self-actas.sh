#!/bin/bash
# Permitir que Compute SA atue como ela mesma

COMPUTE_SA="467992722695-compute@developer.gserviceaccount.com"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔧 Compute SA → Self ActAs Permission"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "Permitindo Compute SA atuar como ela mesma..."
gcloud iam service-accounts add-iam-policy-binding \
  $COMPUTE_SA \
  --member="serviceAccount:$COMPUTE_SA" \
  --role="roles/iam.serviceAccountUser" \
  --condition=None

echo ""
echo "✅ Permissão aplicada!"
