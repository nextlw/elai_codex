# CI/CD Setup - Pipedrive MCP Server

Guia completo para configurar integração contínua e deploy automático no Google Cloud Platform usando Cloud Build + Cloud Run.

## 📋 Pré-requisitos

- [x] Projeto GCP ativo com faturamento habilitado
- [x] Repositório GitHub: https://github.com/nextlw/pipedrive-mcp.git
- [x] APIs ativas:
  - Cloud Run API
  - Cloud Build API
  - Artifact Registry API
  - Secret Manager API

## 🏗️ Arquitetura

```
GitHub (push) → Cloud Build (trigger) → Docker Build → Artifact Registry → Cloud Run Deploy
```

## 🚀 Configuração Inicial

### 1. Ativar APIs Necessárias

```bash
gcloud services enable run.googleapis.com \
  cloudbuild.googleapis.com \
  artifactregistry.googleapis.com \
  secretmanager.googleapis.com \
  --project=elaihub-prod
```

### 2. Criar Artifact Registry Repository

```bash
gcloud artifacts repositories create pipedrive-mcp \
  --repository-format=docker \
  --location=us-central1 \
  --description="Repositório de imagens Docker para Pipedrive MCP" \
  --project=elaihub-prod
```

### 3. Criar Secret no Secret Manager

```bash
# Criar secret com o token da API Pipedrive
echo -n "SEU_TOKEN_AQUI" | gcloud secrets create pipedrive-api-token \
  --data-file=- \
  --project=elaihub-prod

# Dar permissão ao Cloud Run Service Account
PROJECT_NUMBER=$(gcloud projects describe elaihub-prod --format="value(projectNumber)")

gcloud secrets add-iam-policy-binding pipedrive-api-token \
  --member=serviceAccount:$PROJECT_NUMBER-compute@developer.gserviceaccount.com \
  --role=roles/secretmanager.secretAccessor \
  --project=elaihub-prod
```

### 4. Configurar Cloud Build com GitHub

#### Opção A: Via Console (Recomendado)

1. Acesse: https://console.cloud.google.com/cloud-build/triggers
2. Clique em **"Criar gatilho"**
3. Configure:
   - **Nome:** `pipedrive-mcp-deploy`
   - **Evento:** Push para uma ramificação
   - **Fonte:** Conecte seu repositório GitHub
   - **Repositório:** `nextlw/pipedrive-mcp`
   - **Ramificação:** `^main$`
   - **Configuração:** Cloud Build (cloudbuild.yaml)
   - **Localização:** `/cloudbuild.yaml`

#### Opção B: Via CLI

```bash
# Primeiro, conecte o repositório GitHub no console
# Depois crie o gatilho:

gcloud builds triggers create github \
  --name="pipedrive-mcp-deploy" \
  --repo-name="pipedrive-mcp" \
  --repo-owner="nextlw" \
  --branch-pattern="^main$" \
  --build-config="cloudbuild.yaml" \
  --project=elaihub-prod
```

### 5. Dar Permissões ao Cloud Build Service Account

```bash
PROJECT_NUMBER=$(gcloud projects describe elaihub-prod --format="value(projectNumber)")

# Permissão para deploy no Cloud Run
gcloud projects add-iam-policy-binding elaihub-prod \
  --member=serviceAccount:$PROJECT_NUMBER@cloudbuild.gserviceaccount.com \
  --role=roles/run.admin

# Permissão para atuar como service account
gcloud iam service-accounts add-iam-policy-binding \
  $PROJECT_NUMBER-compute@developer.gserviceaccount.com \
  --member=serviceAccount:$PROJECT_NUMBER@cloudbuild.gserviceaccount.com \
  --role=roles/iam.serviceAccountUser \
  --project=elaihub-prod
```

## 🔄 Fluxo de Deploy Automático

### Quando acontece?

Todo **push na branch `main`** dispara automaticamente:

1. **Build:** Cloud Build cria imagem Docker
2. **Tag:** Imagem recebe duas tags:
   - `latest` (sempre a versão mais recente)
   - `$COMMIT_SHA` (hash do commit para rastreabilidade)
3. **Push:** Imagem enviada ao Artifact Registry
4. **Deploy:** Cloud Run atualiza serviço com nova imagem

### Acompanhar Build

```bash
# Listar builds recentes
gcloud builds list --limit=10 --project=elaihub-prod

# Ver logs de um build específico
gcloud builds log BUILD_ID --project=elaihub-prod

# Stream logs em tempo real
gcloud builds log $(gcloud builds list --limit=1 --format="value(id)") --stream --project=elaihub-prod
```

## 📝 Arquivo cloudbuild.yaml

O arquivo [`cloudbuild.yaml`](./cloudbuild.yaml) na raiz do projeto define os passos do build:

```yaml
steps:
  # 1. Build da imagem
  - name: 'gcr.io/cloud-builders/docker'
    args: ['build', '-t', '...', '.']

  # 2. Push para Artifact Registry
  - name: 'gcr.io/cloud-builders/docker'
    args: ['push', '--all-tags', '...']

  # 3. Deploy no Cloud Run
  - name: 'gcr.io/google.com/cloudsdktool/cloud-sdk'
    entrypoint: gcloud
    args: ['run', 'deploy', 'pipedrive-mcp', ...]
```

## 🧪 Testar Pipeline

### 1. Build Manual (Teste Local)

```bash
cd /Users/williamduarte/NCMproduto/elaiRoo/packages/mcp/src/mcp_pipedrive

# Submeter build manualmente
gcloud builds submit \
  --config=cloudbuild.yaml \
  --project=elaihub-prod
```

### 2. Testar Gatilho Automático

```bash
# Fazer uma mudança simples
echo "# CI/CD Test" >> README.md

git add .
git commit -m "test: validar pipeline CI/CD"
git push origin main

# Acompanhar build
gcloud builds list --limit=1 --project=elaihub-prod
```

### 3. Verificar Deploy

```bash
# Ver revisões do serviço
gcloud run revisions list \
  --service=pipedrive-mcp \
  --region=us-central1 \
  --project=elaihub-prod

# Testar endpoint
curl https://pipedrive-mcp-467992722695.us-central1.run.app/sse
```

## 🔧 Configuração do Cloud Run

### Variáveis de Ambiente

Definidas no `cloudbuild.yaml`:

```bash
--set-env-vars=TRANSPORT=sse,CONTAINER_MODE=true,PIPEDRIVE_COMPANY_DOMAIN=suri,HOST=0.0.0.0,PORT=8080
```

**Importante:** Cloud Run injeta automaticamente a variável `PORT` (sempre 8080). Não é necessário defini-la manualmente.

### Secrets

```bash
--set-secrets=PIPEDRIVE_API_TOKEN=pipedrive-api-token:latest
```

O token da API é injetado de forma segura via Secret Manager.

### Recursos e Escala

```bash
--memory=512Mi              # Memória por instância
--cpu=1                     # CPUs por instância
--timeout=300               # Timeout de 5 minutos
--concurrency=80            # Requisições simultâneas
--min-instances=0           # Scale to zero
--max-instances=5           # Máximo de instâncias
```

## 🚦 Rollback

### Ver Revisões

```bash
gcloud run revisions list \
  --service=pipedrive-mcp \
  --region=us-central1 \
  --project=elaihub-prod
```

### Fazer Rollback

```bash
# Voltar para revisão específica
gcloud run services update-traffic pipedrive-mcp \
  --to-revisions=pipedrive-mcp-00042-abc=100 \
  --region=us-central1 \
  --project=elaihub-prod
```

### Rollback para Imagem Específica

```bash
# Deploy com imagem de commit anterior
gcloud run services update pipedrive-mcp \
  --image=us-central1-docker.pkg.dev/elaihub-prod/pipedrive-mcp/server:COMMIT_SHA_ANTERIOR \
  --region=us-central1 \
  --project=elaihub-prod
```

## 📊 Monitoramento

### Logs do Cloud Build

```bash
# Logs do build mais recente
gcloud builds log $(gcloud builds list --limit=1 --format="value(id)") --project=elaihub-prod
```

### Logs do Cloud Run

```bash
# Logs recentes
gcloud run services logs read pipedrive-mcp \
  --region=us-central1 \
  --project=elaihub-prod \
  --limit=50

# Stream logs em tempo real
gcloud run services logs tail pipedrive-mcp \
  --region=us-central1 \
  --project=elaihub-prod
```

### Métricas no Console

- **Cloud Build:** https://console.cloud.google.com/cloud-build/builds?project=elaihub-prod
- **Cloud Run:** https://console.cloud.google.com/run/detail/us-central1/pipedrive-mcp/metrics?project=elaihub-prod
- **Artifact Registry:** https://console.cloud.google.com/artifacts/docker/elaihub-prod/us-central1/pipedrive-mcp?project=elaihub-prod

## 🐛 Troubleshooting

### Build Falha: "permission denied"

```bash
# Verificar permissões do Cloud Build Service Account
PROJECT_NUMBER=$(gcloud projects describe elaihub-prod --format="value(projectNumber)")

gcloud projects get-iam-policy elaihub-prod \
  --flatten="bindings[].members" \
  --filter="bindings.members:serviceAccount:$PROJECT_NUMBER@cloudbuild.gserviceaccount.com"
```

### Deploy Falha: "revision failed"

```bash
# Ver logs da revisão que falhou
gcloud run revisions describe REVISION_NAME \
  --region=us-central1 \
  --project=elaihub-prod
```

### Imagem não encontrada

```bash
# Listar imagens no Artifact Registry
gcloud artifacts docker images list \
  us-central1-docker.pkg.dev/elaihub-prod/pipedrive-mcp \
  --project=elaihub-prod
```

### Secret não acessível

```bash
# Verificar permissões do secret
gcloud secrets get-iam-policy pipedrive-api-token --project=elaihub-prod

# Adicionar permissão se necessário
PROJECT_NUMBER=$(gcloud projects describe elaihub-prod --format="value(projectNumber)")

gcloud secrets add-iam-policy-binding pipedrive-api-token \
  --member=serviceAccount:$PROJECT_NUMBER-compute@developer.gserviceaccount.com \
  --role=roles/secretmanager.secretAccessor \
  --project=elaihub-prod
```

## 💰 Custos Estimados

### Cloud Build

- **Tempo de build:** ~3-5 minutos
- **Custo:** Primeiros 120 minutos/dia grátis, depois $0.003/minuto
- **Estimativa:** ~$2-5/mês (dependendo da frequência)

### Artifact Registry

- **Storage:** ~$0.10/GB/mês
- **Estimativa:** ~$0.50/mês (5GB de imagens)

### Cloud Run

- **Instâncias ativas:** ~$0.00002400/segundo
- **Memória:** ~$0.0000025/GB/segundo
- **Requisições:** $0.40 por 1 milhão
- **Estimativa:** < $5/mês com scale to zero

### Secret Manager

- **Secrets ativos:** $0.06/mês por secret
- **Acesso:** $0.03 por 10.000 acessos
- **Estimativa:** ~$0.10/mês

**Total estimado:** ~$5-10/mês

## 🔐 Segurança

### Secrets

- ✅ Token da API armazenado no Secret Manager
- ✅ Nunca commitado no código
- ✅ Acesso restrito via IAM

### Imagens

- ✅ Armazenadas no Artifact Registry privado
- ✅ Versionadas por commit SHA
- ✅ Auditoria completa de acesso

### Cloud Run

- ✅ HTTPS obrigatório
- ✅ Pode ser protegido com IAM (atualmente `--allow-unauthenticated` para testes)
- ✅ Isolamento por container

### Para Produção

```bash
# Remover acesso público
gcloud run services update pipedrive-mcp \
  --no-allow-unauthenticated \
  --region=us-central1 \
  --project=elaihub-prod

# Dar acesso apenas a service accounts específicas
gcloud run services add-iam-policy-binding pipedrive-mcp \
  --member=serviceAccount:sua-sa@elaihub-prod.iam.gserviceaccount.com \
  --role=roles/run.invoker \
  --region=us-central1 \
  --project=elaihub-prod
```

## 📚 Referências

- [Cloud Build - Deploy para Cloud Run](https://cloud.google.com/build/docs/deploying-builds/deploy-cloud-run)
- [Cloud Run - Continuous Deployment](https://cloud.google.com/run/docs/continuous-deployment-with-cloud-build)
- [Artifact Registry - Docker](https://cloud.google.com/artifact-registry/docs/docker)
- [Secret Manager - Best Practices](https://cloud.google.com/secret-manager/docs/best-practices)
- [Punk do DevOps - CI/CD com Cloud Build](http://punkdodevops.com/2023/07/20/implementacao-continua-com-cloud-build-cloud-run-e-docker/)

## ✅ Checklist Final

- [x] APIs ativadas
- [x] Artifact Registry criado
- [x] Secret Manager configurado
- [x] Dockerfile ajustado para Cloud Run
- [x] cloudbuild.yaml criado
- [ ] Cloud Build conectado ao GitHub
- [ ] Gatilho configurado
- [ ] Permissões do Service Account configuradas
- [ ] Primeiro build manual testado
- [ ] Deploy automático testado
- [ ] Monitoramento configurado

---

**Última atualização:** 06/10/2025
