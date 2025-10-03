# Docker Build - Checklist de Verificação

## ✅ Problemas Já Resolvidos

1. ✅ **Workspace conflitante do CLI**
   - Solução: Adicionado `cli/` ao `.dockerignore`

2. ✅ **Paths corretos dos binários**
   - Build a partir de `/build` (workspace raiz)
   - Binários em `/build/target/release/`

3. ✅ **Sandbox Linux configurado**
   - libseccomp2 instalado
   - Debian Bookworm com kernel 6.1+

## ⚠️ Possíveis Problemas que Podem Ocorrer

### 1. Tempo de Build Muito Longo
**Causa:** Build completo do workspace é pesado (30+ crates)
**Solução:**
- Primeira build pode levar 20-40 minutos
- Use cache do Docker: `docker build --cache-from codex-wrapper:latest`

### 2. Falta de Memória durante Build
**Causa:** Rust precisa de muita RAM para compilar
**Solução:**
- Aumente RAM do Docker para 8GB+: Docker Desktop → Settings → Resources
- Ou use `CARGO_BUILD_JOBS=2` para limitar paralelismo

### 3. Erro "not found" em algum crate
**Causa:** Crate pode estar faltando na lista do Dockerfile
**Solução:** Verificar lista de crates do workspace vs Dockerfile

### 4. Erro de permissão ao executar
**Causa:** Usuário `codex` sem permissões
**Solução:** Já configurado no Dockerfile (USER codex, chmod +x)

### 5. Cloud Run não consegue baixar secrets
**Causa:** Credenciais não configuradas
**Solução:** Use Google Secret Manager conforme README

## 🔍 Verificação Rápida Antes do Build

Execute estes comandos para validar:

```bash
# 1. Verificar se todos os crates existem
cd /Users/williamduarte/NCMproduto/codex/codex-rs
for crate in ansi-escape app-server apply-patch arg0 backend-client chatgpt cli cloud-tasks cloud-tasks-client codex-backend-openapi-models common core exec execpolicy file-search git-apply git-tooling linux-sandbox login mcp-client mcp-server mcp-types ollama otel process-hardening protocol protocol-ts responses-api-proxy rmcp-client tui utils wrapper-cloud-run; do
  if [ ! -d "$crate" ]; then
    echo "❌ Faltando: $crate"
  fi
done

# 2. Verificar se Cargo.toml/Lock existem
test -f Cargo.toml && echo "✅ Cargo.toml OK" || echo "❌ Cargo.toml FALTANDO"
test -f Cargo.lock && echo "✅ Cargo.lock OK" || echo "❌ Cargo.lock FALTANDO"

# 3. Verificar rust-toolchain
test -f rust-toolchain -o -f rust-toolchain.toml && echo "✅ rust-toolchain OK" || echo "⚠️  rust-toolchain ausente (não crítico)"

# 4. Verificar .dockerignore
test -f wrapper-cloud-run/.dockerignore && echo "✅ .dockerignore OK" || echo "❌ .dockerignore FALTANDO"
```

## 📦 Comando de Build Recomendado

```bash
# Build normal
cd /Users/williamduarte/NCMproduto/codex/codex-rs
docker build \
  -f wrapper-cloud-run/Dockerfile \
  -t codex-wrapper:latest \
  --progress=plain \
  .

# Build com limite de memória (se necessário)
docker build \
  -f wrapper-cloud-run/Dockerfile \
  -t codex-wrapper:latest \
  --progress=plain \
  --build-arg CARGO_BUILD_JOBS=2 \
  .
```

## 🧪 Teste Rápido Após Build

```bash
# 1. Verificar se imagem foi criada
docker images | grep codex-wrapper

# 2. Testar healthcheck
docker run --rm -d -p 8080:8080 \
  -e GATEWAY_API_KEY="test" \
  --name codex-test \
  codex-wrapper:latest

# Aguardar 5s
sleep 5

# Testar
curl http://localhost:8080/health

# Parar
docker stop codex-test
```

## 🚀 Deploy no Cloud Run

Ver instruções completas no README.md, seção "Deploy em Nuvem"

## 📝 Notas Importantes

- O `.dockerignore` exclui `cli/` para evitar conflito de workspace
- Primeira build é lenta, mas rebuilds aproveitam cache
- O wrapper procura `./codex-app-server` no mesmo diretório
- Sandbox requer kernel Linux 5.13+ (Debian Bookworm tem 6.1+)
