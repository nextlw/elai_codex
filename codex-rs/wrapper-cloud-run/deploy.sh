#!/bin/bash
set -e

# Fix Python path for gcloud
export CLOUDSDK_PYTHON=/usr/bin/python3

echo "🚀 Building and deploying codex-wrapper with MCP Pipedrive..."

# Set build context to parent directory (codex-rs)
cd /Users/williamduarte/NCMproduto/codex/codex-rs

# Deploy to Cloud Run
gcloud run deploy codex-wrapper \
  --source wrapper-cloud-run \
  --region us-central1 \
  --platform managed \
  --allow-unauthenticated \
  --memory 2Gi \
  --cpu 2 \
  --timeout 3600 \
  --max-instances 10 \
  --set-env-vars RUST_LOG=info \
  --set-env-vars PIPEDRIVE_API_TOKEN=d406f484f23c9ccb32281d73d445c03dd240f3ec

echo "✅ Deploy completed!"
echo "📋 Check service at: https://console.cloud.google.com/run?project=buzzlightear"
