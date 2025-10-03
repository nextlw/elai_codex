#!/bin/bash

API_KEY="IxF3WoAB6IBrNJKrC/Jsr5yjt2bXHZkBSHFDBhcIVvc="
CLI="./cli/target/release/codex-wrapper-cli"

echo "=========================================="
echo "Testando Codex Wrapper CLI"
echo "=========================================="
echo ""

# Verifica se o CLI foi compilado
if [ ! -f "$CLI" ]; then
    echo "❌ CLI não encontrado. Compilando..."
    cd cli && cargo build --release && cd ..
fi

echo "✅ CLI encontrado: $CLI"
echo ""

# Verifica se o wrapper está rodando
echo "🔍 Verificando wrapper..."
if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Wrapper está rodando em http://localhost:8080"
else
    echo "❌ Wrapper não está rodando!"
    echo "Execute: cargo run --release"
    exit 1
fi
echo ""

echo "=========================================="
echo "Teste 1: Help do CLI"
echo "=========================================="
$CLI --help
echo ""

echo "=========================================="
echo "Teste 2: Comando simples (com autenticação)"
echo "=========================================="
$CLI --api-key "$API_KEY" echo "Hello from Codex CLI"
echo ""

echo "=========================================="
echo "Teste 3: Sem autenticação (deve falhar)"
echo "=========================================="
$CLI echo "This should fail" 2>&1 | head -5 || echo "❌ Falhou como esperado (401)"
echo ""

echo "=========================================="
echo "Teste 4: Modo interativo"
echo "=========================================="
echo "Para testar o modo interativo, execute:"
echo "  $CLI --api-key \"$API_KEY\""
echo ""
echo "Depois digite comandos como:"
echo "  codex> echo Hello"
echo "  codex> exit"
echo ""

echo "✅ Testes concluídos!"
