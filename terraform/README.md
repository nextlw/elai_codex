# Infraestrutura Terraform para Codex Gateway

Este diretório contém a configuração Terraform para provisionar a infraestrutura GCP necessária para o Codex Gateway.

## Recursos Criados

1. **Bucket Cloud Storage** - Para armazenar artefatos gerados
   - Versionamento habilitado
   - Políticas de ciclo de vida (retenção de 30 dias)
   - Permissões IAM para service account

2. **Banco Firestore** - Para gerenciamento de API keys e sessões
   - Modo nativo
   - Concorrência otimista
   - Implantado na região especificada

3. **Secrets do Secret Manager** - Para credenciais sensíveis
   - Gateway API key
   - Anthropic API key
   - OpenAI API key (opcional)
   - Pipedrive API token (opcional)

4. **Permissões IAM** - Controles de acesso adequados
   - Acesso da service account aos secrets
   - Acesso da service account ao bucket de storage

5. **Cloud Monitoring** - Métricas baseadas em logs
   - Métricas de requisições API
   - Rastreamento de erros

## Pré-requisitos

1. **Instalar Terraform** (>= 1.5.0)
   ```bash
   brew install terraform  # macOS
   # ou baixe de https://www.terraform.io/downloads
   ```

2. **Autenticar com GCP**
   ```bash
   gcloud auth application-default login
   gcloud config set project elaihub-prod
   ```

3. **Habilitar APIs necessárias**
   ```bash
   gcloud services enable \
     cloudresourcemanager.googleapis.com \
     serviceusage.googleapis.com \
     storage.googleapis.com \
     firestore.googleapis.com \
     secretmanager.googleapis.com \
     logging.googleapis.com \
     monitoring.googleapis.com
   ```

## Uso

### 1. Inicializar Terraform

```bash
cd terraform
terraform init
```

### 2. Revisar o plano

```bash
terraform plan
```

### 3. Aplicar a configuração

```bash
terraform apply
```

Revise as mudanças e digite `yes` para confirmar.

### 4. Configurar secrets

Após aplicar, você precisa adicionar os valores reais dos secrets:

```bash
# Gateway API Key
echo -n "sua-gateway-api-key" | \
  gcloud secrets versions add gateway-api-key --data-file=-

# Anthropic API Key
echo -n "sk-ant-sua-key" | \
  gcloud secrets versions add anthropic-api-key --data-file=-

# OpenAI API Key (opcional)
echo -n "sk-sua-openai-key" | \
  gcloud secrets versions add openai-api-key --data-file=-

# Pipedrive API Token (opcional)
echo -n "seu-pipedrive-token" | \
  gcloud secrets versions add pipedrive-api-token --data-file=-
```

## Personalização

Copie `terraform.tfvars.example` para `terraform.tfvars` e ajuste os valores:

```bash
cp terraform.tfvars.example terraform.tfvars
```

Edite `terraform.tfvars` com seus valores específicos.

## Gerenciamento de Estado

Por padrão, o estado do Terraform é armazenado localmente. Para produção, considere usar um backend remoto:

1. Criar um bucket GCS para o estado:
   ```bash
   gsutil mb gs://elaihub-prod-terraform-state
   gsutil versioning set on gs://elaihub-prod-terraform-state
   ```

2. Descomentar a configuração de backend em `main.tf`

3. Inicializar com o backend:
   ```bash
   terraform init -migrate-state
   ```

## Outputs

Após aplicar, o Terraform irá exibir informações importantes:

- `artifacts_bucket_name` - Nome do bucket GCS
- `artifacts_bucket_url` - URL do bucket GCS
- `firestore_database_name` - Nome do banco Firestore
- `secret_ids` - IDs dos secrets criados

Visualize os outputs a qualquer momento com:
```bash
terraform output
```

## Limpeza

Para destruir todos os recursos (use com cuidado):

```bash
terraform destroy
```

## Solução de Problemas

### Erros de Permissão Negada

Certifique-se de que sua conta GCP tem as seguintes roles:
- Storage Admin
- Firestore Admin
- Secret Manager Admin
- Logging Admin
- Monitoring Admin

### Firestore Já Existe

Se o Firestore já estiver configurado no seu projeto, comente o recurso `google_firestore_database` em `main.tf`.

### API Não Habilitada

Habilite as APIs necessárias:
```bash
gcloud services enable firestore.googleapis.com
gcloud services enable secretmanager.googleapis.com
```

## Melhores Práticas

1. **Use ambientes separados** - Crie workspaces ou diretórios diferentes do Terraform para prod/staging
2. **Controle de versão** - Faça commit dos arquivos terraform (mas não `.tfstate` ou `.tfvars` com secrets)
3. **Revise mudanças** - Sempre execute `terraform plan` antes de `apply`
4. **Backup de estado** - Mantenha backups do seu estado Terraform
5. **Rotação de secrets** - Rotacione regularmente os secrets no Secret Manager

## Recursos Adicionais

- [Documentação do Provider GCP para Terraform](https://registry.terraform.io/providers/hashicorp/google/latest/docs)
- [GCP Secret Manager](https://cloud.google.com/secret-manager/docs)
- [GCP Firestore](https://cloud.google.com/firestore/docs)
- [GCP Cloud Storage](https://cloud.google.com/storage/docs)
