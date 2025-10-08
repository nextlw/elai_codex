#!/bin/bash

# Script para configurar permissões do Cloud Build
# Execute este script para dar permissões ao Cloud Build para usar Artifact Registry e Cloud Run

set -e

# Cores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Configurando permissões do Cloud Build...${NC}\n"

# Obter PROJECT_ID
PROJECT_ID=$(gcloud config get-value project 2>/dev/null)

if [ -z "$PROJECT_ID" ]; then
    echo -e "${RED}Erro: Não foi possível obter o PROJECT_ID${NC}"
    echo "Execute: gcloud config set project SEU_PROJECT_ID"
    exit 1
fi

echo -e "${GREEN}Projeto: ${PROJECT_ID}${NC}\n"

# Obter número do projeto
PROJECT_NUMBER=$(gcloud projects describe $PROJECT_ID --format='value(projectNumber)')
CLOUD_BUILD_SA="${PROJECT_NUMBER}@cloudbuild.gserviceaccount.com"

echo -e "${YELLOW}Conta de serviço do Cloud Build: ${CLOUD_BUILD_SA}${NC}\n"

# 1. Habilitar APIs necessárias
echo -e "${YELLOW}1. Habilitando APIs necessárias...${NC}"
gcloud services enable cloudbuild.googleapis.com
gcloud services enable run.googleapis.com
gcloud services enable artifactregistry.googleapis.com
gcloud services enable secretmanager.googleapis.com
echo -e "${GREEN}✓ APIs habilitadas${NC}\n"

# 2. Criar repositório no Artifact Registry (se não existir)
echo -e "${YELLOW}2. Verificando repositório no Artifact Registry...${NC}"
if ! gcloud artifacts repositories describe pipedrive-mcp \
    --location=us-central1 &>/dev/null; then
    echo "Criando repositório pipedrive-mcp..."
    gcloud artifacts repositories create pipedrive-mcp \
        --repository-format=docker \
        --location=us-central1 \
        --description="Docker images for Pipedrive MCP Server"
    echo -e "${GREEN}✓ Repositório criado${NC}\n"
else
    echo -e "${GREEN}✓ Repositório já existe${NC}\n"
fi

# 3. Dar permissão de Artifact Registry Writer
echo -e "${YELLOW}3. Configurando permissão de Artifact Registry Writer...${NC}"
gcloud artifacts repositories add-iam-policy-binding pipedrive-mcp \
    --location=us-central1 \
    --member="serviceAccount:${CLOUD_BUILD_SA}" \
    --role="roles/artifactregistry.writer"
echo -e "${GREEN}✓ Permissão de Artifact Registry Writer configurada${NC}\n"

# 4. Dar permissão de Cloud Run Admin
echo -e "${YELLOW}4. Configurando permissão de Cloud Run Admin...${NC}"
gcloud projects add-iam-policy-binding $PROJECT_ID \
    --member="serviceAccount:${CLOUD_BUILD_SA}" \
    --role="roles/run.admin" \
    --condition=None
echo -e "${GREEN}✓ Permissão de Cloud Run Admin configurada${NC}\n"

# 5. Dar permissão de Service Account User
echo -e "${YELLOW}5. Configurando permissão de Service Account User...${NC}"
gcloud projects add-iam-policy-binding $PROJECT_ID \
    --member="serviceAccount:${CLOUD_BUILD_SA}" \
    --role="roles/iam.serviceAccountUser" \
    --condition=None
echo -e "${GREEN}✓ Permissão de Service Account User configurada${NC}\n"

# 6. Dar permissão de Secret Manager Accessor
echo -e "${YELLOW}6. Configurando permissão de Secret Manager...${NC}"
gcloud projects add-iam-policy-binding $PROJECT_ID \
    --member="serviceAccount:${CLOUD_BUILD_SA}" \
    --role="roles/secretmanager.secretAccessor" \
    --condition=None
echo -e "${GREEN}✓ Permissão de Secret Manager configurada${NC}\n"

# 7. Verificar se o secret existe
echo -e "${YELLOW}7. Verificando secret do Pipedrive...${NC}"
if ! gcloud secrets describe pipedrive-api-token &>/dev/null; then
    echo -e "${RED}⚠ Secret 'pipedrive-api-token' não encontrado${NC}"
    echo -e "${YELLOW}Crie o secret com:${NC}"
    echo "echo -n 'SEU_TOKEN_AQUI' | gcloud secrets create pipedrive-api-token --data-file=-"
    echo ""
else
    echo -e "${GREEN}✓ Secret existe${NC}\n"
fi

echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Configuração concluída com sucesso!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"

echo -e "${YELLOW}Próximos passos:${NC}"
echo -e "1. Se o secret não existe, crie-o com o comando mostrado acima"
echo -e "2. Execute um novo build:"
echo -e "   ${GREEN}gcloud builds submit --config=cloudbuild.yaml${NC}\n"
