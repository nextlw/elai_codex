# Pipedrive MCP - Deploy no Google Cloud Run

## ✅ Deploy Concluído

**Projeto:** `elaihub-prod` (467992722695)
**Região:** `us-central1`
**Serviço:** `pipedrive-mcp`
**URL:** `https://pipedrive-mcp-467992722695.us-central1.run.app`
**Endpoint SSE:** `https://pipedrive-mcp-467992722695.us-central1.run.app/sse`

## 📋 Recursos Criados

### 1. Secret Manager
- **Secret:** `pipedrive-api-token`
- **Valor:** Token da API do Pipedrive (domínio suri)
- **Permissões:** Service Account `467992722695-compute@developer.gserviceaccount.com` tem acesso

### 2. Artifact Registry
- **Repositório:** `pipedrive-mcp`
- **Localização:** `us-central1`
- **Imagem:** `us-central1-docker.pkg.dev/elaihub-prod/pipedrive-mcp/server:latest`

### 3. Cloud Run Service
- **Nome:** `pipedrive-mcp`
- **Memória:** 512Mi
- **CPU:** 1
- **Timeout:** 300s
- **Concurrency:** 80
- **Min Instances:** 0 (scale to zero)
- **Max Instances:** 5

## 🔒 Autenticação

O serviço está **protegido por IAM** devido a políticas de organização.

### Opção 1: Docker Local (Recomendado para desenvolvimento)

**Configuração atual em `packages/mcp/src/mcp.json`:**

```json
"pipedrive-mcp": {
  "command": "docker",
  "args": [
    "run", "-i", "--rm",
    "-e", "PIPEDRIVE_API_TOKEN=d406f484f23c9ccb32281d73d445c03dd240f3ec",
    "-e", "PIPEDRIVE_COMPANY_DOMAIN=suri",
    "-e", "TRANSPORT=stdio",
    "pipedrive-mcp"
  ]
}
```

**Vantagens:**
- ✅ Funciona imediatamente
- ✅ Sem complexidade de autenticação
- ✅ Execução local isolada

### Opção 2: Cloud Run com Autenticação (Produção)

Para usar o endpoint remoto, você precisa configurar autenticação com Identity Token.

#### Service Accounts com Permissão:
- `codex-wrapper-sa@elaihub-prod.iam.gserviceaccount.com` ✅

#### Exemplo com curl:
```bash
TOKEN=$(gcloud auth print-identity-token)

curl -H "Authorization: Bearer $TOKEN" \
  "https://pipedrive-mcp-467992722695.us-central1.run.app/sse"
```

#### Configuração no mcp.json (Exemplo - Requer implementação de auth):
```json
"pipedrive-mcp-remote": {
  "command": "npx",
  "args": [
    "-y",
    "mcp-remote@latest",
    "https://pipedrive-mcp-467992722695.us-central1.run.app/sse"
  ],
  "env": {
    "GOOGLE_APPLICATION_CREDENTIALS": "/path/to/service-account-key.json"
  }
}
```

## 🚀 Como Usar

### 1. Verificar que o Docker local está rodando:
```bash
docker ps | grep pipedrive-mcp
```

### 2. Testar ferramenta via McpHub:
```typescript
import { McpHub } from './src/services/mcp/McpHub.js';

const hub = McpHub.getInstance();
await hub.refreshServers();

// Listar deals
const result = await hub.callTool('list_deals_from_pipedrive', {});
console.log(result);
```

### 3. Verificar logs do Cloud Run:
```bash
gcloud run services logs read pipedrive-mcp \
  --region us-central1 \
  --project=elaihub-prod \
  --limit 50
```

## 🔄 Atualizar Serviço

### Re-deploy com nova versão:
```bash
cd /Users/williamduarte/NCMproduto/elaiRoo/packages/mcp/src/mcp_pipedrive

# Build nova imagem
gcloud builds submit \
  --tag us-central1-docker.pkg.dev/elaihub-prod/pipedrive-mcp/server:v2 \
  --project=elaihub-prod

# Atualizar serviço
gcloud run services update pipedrive-mcp \
  --image us-central1-docker.pkg.dev/elaihub-prod/pipedrive-mcp/server:v2 \
  --region us-central1 \
  --project=elaihub-prod
```

### Ou usar o script:
```bash
./deploy-gcp.sh
```

## 📊 Monitoramento

### Métricas no Console:
https://console.cloud.google.com/run/detail/us-central1/pipedrive-mcp/metrics?project=elaihub-prod

### Logs em tempo real:
```bash
gcloud run services logs tail pipedrive-mcp \
  --region us-central1 \
  --project=elaihub-prod
```

## 💰 Custos Estimados

- **Imagem Docker:** ~$0.05/mês (storage)
- **Secret Manager:** $0.06/mês por secret
- **Cloud Run:**
  - Scale to zero = $0 quando não está em uso
  - ~$0.40 por 1 milhão de requisições
  - ~$0.10 por GB-segundo de memória

**Total estimado:** < $1/mês em uso normal

## 🐛 Troubleshooting

### Erro 403 ao acessar endpoint:
- **Causa:** Políticas de organização bloqueiam acesso público
- **Solução:** Use Docker local ou configure autenticação com service account

### Erro "Permission denied on secret":
```bash
# Verificar permissões
gcloud secrets get-iam-policy pipedrive-api-token --project=elaihub-prod

# Adicionar permissão se necessário
PROJECT_NUMBER=$(gcloud projects describe elaihub-prod --format="value(projectNumber)")
gcloud secrets add-iam-policy-binding pipedrive-api-token \
  --member=serviceAccount:$PROJECT_NUMBER-compute@developer.gserviceaccount.com \
  --role=roles/secretmanager.secretAccessor \
  --project=elaihub-prod
```

### Container não inicia:
```bash
# Ver logs
gcloud run services logs read pipedrive-mcp \
  --region us-central1 \
  --project=elaihub-prod \
  --limit 100

# Verificar revisão
gcloud run revisions list \
  --service=pipedrive-mcp \
  --region=us-central1 \
  --project=elaihub-prod
```

## 📝 Ferramentas Disponíveis

### Deals (8 ferramentas):
- `list_deals_from_pipedrive` - Listar todos os deals
- `get_deal_from_pipedrive` - Obter detalhes de um deal
- `create_deal_in_pipedrive` - Criar novo deal
- `update_deal_in_pipedrive` - Atualizar deal
- `delete_deal_from_pipedrive` - Deletar deal
- `search_deals_in_pipedrive` - Buscar deals
- `update_product_in_deal_in_pipedrive` - Atualizar produto em deal
- `delete_product_from_deal_in_pipedrive` - Remover produto de deal

### Activities (7 ferramentas):
- `create_activity_in_pipedrive`
- `get_activity_from_pipedrive`
- `list_activities_from_pipedrive`
- `update_activity_in_pipedrive`
- `delete_activity_from_pipedrive`
- `get_activity_types_from_pipedrive`
- `create_activity_type_in_pipedrive`

### Organizations (8 ferramentas):
- `create_organization_in_pipedrive`
- `get_organization_from_pipedrive`
- `update_organization_in_pipedrive`
- `delete_organization_from_pipedrive`
- `search_organizations_in_pipedrive`
- `list_organizations_from_pipedrive`
- `add_follower_to_organization_in_pipedrive`
- `delete_follower_from_organization_in_pipedrive`

### Persons (5 ferramentas):
- `create_person_in_pipedrive`
- `get_person_from_pipedrive`
- `update_person_in_pipedrive`
- `delete_person_from_pipedrive`
- `search_persons_in_pipedrive`

### Item Search (2 ferramentas):
- `search_items_in_pipedrive` - Busca global
- `search_item_field_in_pipedrive` - Busca por campo específico

**Total:** 30 ferramentas

---

**Data do Deploy:** 04/10/2025
**Última Atualização:** 04/10/2025
