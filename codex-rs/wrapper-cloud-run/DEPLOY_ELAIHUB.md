# Deploy do Codex Wrapper no ElaiHUB Production

## ✅ Verificação Inicial

**Projeto:** `elaihub-prod` (467992722695)
**Conta:** `adm@nexcode.live`

### Serviços Habilitados:
- ✅ Cloud Run (`run.googleapis.com`)
- ✅ Artifact Registry (`artifactregistry.googleapis.com`)
- ✅ Cloud Storage (`storage.googleapis.com`)
- ✅ Secret Manager (`secretmanager.googleapis.com`)

---

## 📦 Passo 1: Criar Recursos Necessários

### 1.1 Criar Artifact Registry (se não existir)

```bash
gcloud artifacts repositories create codex-wrapper \
  --repository-format=docker \
  --location=us-central1 \
  --description="Codex Wrapper Cloud Run images" \
  --project=elaihub-prod
```

### 1.2 Criar Bucket do Cloud Storage para Sessões (Opcional)

```bash
gcloud storage buckets create gs://elaihub-codex-sessions \
  --project=elaihub-prod \
  --location=us-central1 \
  --uniform-bucket-level-access
```

### 1.3 Criar Secrets no Secret Manager

```bash
# API Key do Gateway (gerar token seguro)
echo -n "$(openssl rand -base64 32)" | \
gcloud secrets create codex-gateway-api-key \
  --data-file=- \
  --project=elaihub-prod

# OpenAI API Key
echo -n "SUA_OPENAI_API_KEY" | \
gcloud secrets create codex-openai-api-key \
  --data-file=- \
  --project=elaihub-prod

# Anthropic API Key
echo -n "SUA_ANTHROPIC_API_KEY" | \
gcloud secrets create codex-anthropic-api-key \
  --data-file=- \
  --project=elaihub-prod
```

---

## 🐳 Passo 2: Build e Push da Imagem Docker

### 2.1 Build Local (Opcional - para testar)

```bash
cd /Users/williamduarte/NCMproduto/codex/codex-rs
docker build -f wrapper-cloud-run/Dockerfile -t codex-wrapper:latest .
```

### 2.2 Build e Push com Cloud Build (Recomendado)

```bash
cd /Users/williamduarte/NCMproduto/codex/codex-rs

gcloud builds submit \
  --tag us-central1-docker.pkg.dev/elaihub-prod/codex-wrapper/codex-wrapper:latest \
  --project=elaihub-prod \
  --timeout=1h \
  -f wrapper-cloud-run/Dockerfile \
  .
```

**Nota:** O build vai demorar ~30-40 min na primeira vez.

---

## 🚀 Passo 3: Deploy no Cloud Run

### 3.1 Deploy Básico

```bash
gcloud run deploy codex-wrapper \
  --image us-central1-docker.pkg.dev/elaihub-prod/codex-wrapper/codex-wrapper:latest \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated \
  --memory 2Gi \
  --cpu 2 \
  --timeout 300 \
  --concurrency 80 \
  --min-instances 0 \
  --max-instances 10 \
  --set-env-vars RUST_LOG=info \
  --set-secrets GATEWAY_API_KEY=codex-gateway-api-key:latest,OPENAI_API_KEY=codex-openai-api-key:latest,ANTHROPIC_API_KEY=codex-anthropic-api-key:latest \
  --project=elaihub-prod
```

### 3.2 Com Storage para Sessões (Opcional)

```bash
gcloud run deploy codex-wrapper \
  --image us-central1-docker.pkg.dev/elaihub-prod/codex-wrapper/codex-wrapper:latest \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated \
  --memory 2Gi \
  --cpu 2 \
  --timeout 300 \
  --concurrency 80 \
  --min-instances 0 \
  --max-instances 10 \
  --set-env-vars RUST_LOG=info,GCS_SESSION_BUCKET=elaihub-codex-sessions \
  --set-secrets GATEWAY_API_KEY=codex-gateway-api-key:latest,OPENAI_API_KEY=codex-openai-api-key:latest,ANTHROPIC_API_KEY=codex-anthropic-api-key:latest \
  --project=elaihub-prod
```

---

## 🧪 Passo 4: Testar o Deploy

### 4.1 Obter URL do Serviço

```bash
gcloud run services describe codex-wrapper \
  --region us-central1 \
  --project=elaihub-prod \
  --format="value(status.url)"
```

### 4.2 Testar Healthcheck

```bash
SERVICE_URL=$(gcloud run services describe codex-wrapper --region us-central1 --project=elaihub-prod --format="value(status.url)")
curl $SERVICE_URL/health
# Deve retornar: OK
```

### 4.3 Obter API Key

```bash
gcloud secrets versions access latest \
  --secret=codex-gateway-api-key \
  --project=elaihub-prod
```

### 4.4 Testar Request Autenticado

```bash
API_KEY=$(gcloud secrets versions access latest --secret=codex-gateway-api-key --project=elaihub-prod)

curl -N -X POST "$SERVICE_URL/api/v1/exec/stream" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"prompt": "echo Hello from ElaiHUB Production"}'
```

---

## 📋 Configurações Recomendadas

### Variáveis de Ambiente

| Variável | Valor | Descrição |
|----------|-------|-----------|
| `PORT` | 8080 | Porta HTTP (padrão Cloud Run) |
| `RUST_LOG` | info | Nível de log |
| `GCS_SESSION_BUCKET` | elaihub-codex-sessions | Bucket para persistência (opcional) |

### Secrets

| Secret | Nome no Secret Manager | Descrição |
|--------|----------------------|-----------|
| `GATEWAY_API_KEY` | codex-gateway-api-key | Token de autenticação do wrapper |
| `OPENAI_API_KEY` | codex-openai-api-key | API Key OpenAI |
| `ANTHROPIC_API_KEY` | codex-anthropic-api-key | API Key Anthropic |

### Recursos Cloud Run

| Recurso | Valor Recomendado | Observação |
|---------|------------------|------------|
| CPU | 2 | Para processamento do app-server |
| Memory | 2Gi | Rust precisa de RAM |
| Timeout | 300s | Para comandos longos |
| Concurrency | 80 | Máx requests simultâneos por instância |
| Min Instances | 0 | Scale to zero (economia) |
| Max Instances | 10 | Controle de custos |

---

## 🔒 Segurança

### ⚠️ IMPORTANTE:

1. **NUNCA** commit API keys no código
2. **SEMPRE** use Secret Manager para credenciais
3. **CONFIGURE** autenticação com `GATEWAY_API_KEY`
4. **MONITORE** uso e custos no Cloud Console
5. **REVISE** logs regularmente

### Permissões IAM Necessárias

O Cloud Run precisa acesso aos secrets:

```bash
PROJECT_NUMBER=$(gcloud projects describe elaihub-prod --format="value(projectNumber)")

gcloud secrets add-iam-policy-binding codex-gateway-api-key \
  --member=serviceAccount:$PROJECT_NUMBER-compute@developer.gserviceaccount.com \
  --role=roles/secretmanager.secretAccessor \
  --project=elaihub-prod

gcloud secrets add-iam-policy-binding codex-openai-api-key \
  --member=serviceAccount:$PROJECT_NUMBER-compute@developer.gserviceaccount.com \
  --role=roles/secretmanager.secretAccessor \
  --project=elaihub-prod

gcloud secrets add-iam-policy-binding codex-anthropic-api-key \
  --member=serviceAccount:$PROJECT_NUMBER-compute@developer.gserviceaccount.com \
  --role=roles/secretmanager.secretAccessor \
  --project=elaihub-prod
```

---

## 📊 Monitoramento

### Logs

```bash
gcloud run services logs read codex-wrapper \
  --region us-central1 \
  --project=elaihub-prod \
  --limit 50
```

### Métricas

Acesse: https://console.cloud.google.com/run/detail/us-central1/codex-wrapper/metrics?project=elaihub-prod

---

## 🔄 Atualização do Serviço

Para atualizar com nova imagem:

```bash
# 1. Build nova imagem
gcloud builds submit \
  --tag us-central1-docker.pkg.dev/elaihub-prod/codex-wrapper/codex-wrapper:v2 \
  --project=elaihub-prod \
  -f wrapper-cloud-run/Dockerfile \
  .

# 2. Atualizar serviço
gcloud run services update codex-wrapper \
  --image us-central1-docker.pkg.dev/elaihub-prod/codex-wrapper/codex-wrapper:v2 \
  --region us-central1 \
  --project=elaihub-prod
```

---

## 📝 Checklist de Deploy

- [ ] Serviços habilitados (Run, Artifact Registry, Storage, Secret Manager)
- [ ] Artifact Registry criado (`codex-wrapper`)
- [ ] Bucket criado (opcional: `elaihub-codex-sessions`)
- [ ] Secrets criados (gateway, openai, anthropic)
- [ ] Permissões IAM configuradas
- [ ] Docker build concluído com sucesso
- [ ] Imagem pushed para Artifact Registry
- [ ] Cloud Run service deployed
- [ ] Healthcheck testado e funcionando
- [ ] Request autenticado testado
- [ ] Monitoramento configurado

---

## 🆘 Troubleshooting

### Erro: "Permission denied" ao acessar secrets
- Verifique permissões IAM (passo de Segurança)

### Erro: "Service unavailable"
- Verifique logs: `gcloud run services logs read codex-wrapper`
- Verifique se o app-server foi copiado corretamente

### Build muito lento
- Normal! Primeira build demora 30-40 min
- Builds subsequentes usam cache e são mais rápidas

### Custos altos
- Reduza `max-instances`
- Configure `min-instances=0` para scale to zero
- Use regiões mais baratas (us-central1 é OK)

---

**Última atualização:** 2025-09-30
**Projeto:** ElaiHUB Production (`elaihub-prod`)
**Região:** us-central1
