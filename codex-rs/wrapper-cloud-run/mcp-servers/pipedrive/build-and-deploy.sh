#!/bin/bash

# Script para build e deploy do Pipedrive MCP Server
# Compatível com Mac M1/M2 (ARM64) e Intel (AMD64)

set -e

# Cores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}🚀 Build e Deploy do Pipedrive MCP Server${NC}\n"

# Obter PROJECT_ID
PROJECT_ID=$(gcloud config get-value project 2>/dev/null)
IMAGE_NAME="us-central1-docker.pkg.dev/${PROJECT_ID}/pipedrive-mcp/server:latest"

echo -e "${GREEN}Projeto: ${PROJECT_ID}${NC}"
echo -e "${GREEN}Imagem: ${IMAGE_NAME}${NC}\n"

# 1. Configurar Docker Buildx (para multi-plataforma)
echo -e "${YELLOW}1. Configurando Docker Buildx...${NC}"
if ! docker buildx ls | grep -q multiplatform; then
    docker buildx create --name multiplatform --driver docker-container --use
    echo -e "${GREEN}✓ Builder multiplatform criado${NC}\n"
else
    docker buildx use multiplatform
    echo -e "${GREEN}✓ Builder multiplatform já existe${NC}\n"
fi

# 2. Build da imagem para linux/amd64 (compatível com Cloud Run)
echo -e "${YELLOW}2. Construindo imagem Docker para linux/amd64...${NC}"
docker buildx build \
    --platform linux/amd64 \
    -t ${IMAGE_NAME} \
    --push \
    .

echo -e "${GREEN}✓ Imagem construída e enviada com sucesso${NC}\n"

# 3. Deploy no Cloud Run
echo -e "${YELLOW}3. Fazendo deploy no Cloud Run...${NC}"
gcloud run deploy pipedrive-mcp \
    --image=${IMAGE_NAME} \
    --region=us-central1 \
    --platform=managed \
    --memory=512Mi \
    --cpu=1 \
    --timeout=300 \
    --concurrency=80 \
    --min-instances=0 \
    --max-instances=5 \
    --set-env-vars=TRANSPORT=sse,CONTAINER_MODE=true,PIPEDRIVE_COMPANY_DOMAIN=suri,HOST=0.0.0.0,PORT=8080 \
    --set-secrets=PIPEDRIVE_API_TOKEN=pipedrive-api-token:latest \
    --allow-unauthenticated

echo -e "\n${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Deploy concluído com sucesso!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"

# Obter URL do serviço
SERVICE_URL=$(gcloud run services describe pipedrive-mcp --region=us-central1 --format='value(status.url)')
echo -e "${YELLOW}URL do serviço:${NC} ${GREEN}${SERVICE_URL}${NC}\n"
