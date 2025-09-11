## Avançado

## Modo não interativo / CI

Execute o Codex em modo head-less em pipelines. Exemplo de etapa no GitHub Action:

```yaml
- name: Atualizar changelog via Codex
  run: |
    npm install -g @openai/codex
    export OPENAI_API_KEY="${{ secrets.OPENAI_KEY }}"
    codex exec --full-auto "update CHANGELOG for next release"
```

## Rastreamento / logging detalhado

Como o Codex é escrito em Rust, ele respeita a variável de ambiente `RUST_LOG` para configurar seu comportamento de logging.

O TUI padrão usa `RUST_LOG=codex_core=info,codex_tui=info` e as mensagens de log são gravadas em `~/.codex/log/codex-tui.log`, então você pode deixar o seguinte rodando em um terminal separado para monitorar as mensagens conforme são escritas:

```
tail -F ~/.codex/log/codex-tui.log
```

Em comparação, o modo não interativo (`codex exec`) usa por padrão `RUST_LOG=error`, mas as mensagens são impressas inline, então não há necessidade de monitorar um arquivo separado.

Consulte a documentação do Rust sobre [`RUST_LOG`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) para mais informações sobre as opções de configuração.

## Model Context Protocol (MCP)

O Codex CLI pode ser configurado para usar servidores MCP definindo uma seção [`mcp_servers`](./config.md#mcp_servers) em `~/.codex/config.toml`. A intenção é espelhar como ferramentas como Claude e Cursor definem `mcpServers` em seus respectivos arquivos JSON de configuração, embora o formato do Codex seja ligeiramente diferente, pois usa TOML ao invés de JSON, por exemplo:

```toml
# IMPORTANTE: a chave de nível superior é `mcp_servers` e não `mcpServers`.
[mcp_servers.server-name]
command = "npx"
args = ["-y", "mcp-server"]
env = { "API_KEY" = "value" }
```

> [!TIP]
> É um recurso um pouco experimental, mas o Codex CLI também pode ser executado como um _servidor_ MCP via `codex mcp`. Se você iniciá-lo com um cliente MCP como `npx @modelcontextprotocol/inspector codex mcp` e enviar uma requisição `tools/list`, verá que há apenas uma ferramenta, `codex`, que aceita uma variedade de entradas, incluindo um mapa `config` abrangente para qualquer configuração que você queira sobrescrever. Sinta-se à vontade para experimentar e enviar feedback via issues no GitHub.