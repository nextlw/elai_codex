# Config

O Codex suporta vários mecanismos para definir valores de configuração:

- Flags específicas de configuração na linha de comando, como `--model o3` (maior precedência).
- Uma flag genérica `-c`/`--config` que aceita um par `key=value`, como `--config model="o3"`.
  - A chave pode conter pontos para definir um valor mais profundo que a raiz, por exemplo, `--config model_providers.openai.wire_api="chat"`.
  - Os valores podem conter objetos, como `--config shell_environment_policy.include_only=["PATH", "HOME", "USER"]`.
  - Para consistência com `config.toml`, os valores estão no formato TOML em vez de JSON, então use `{a = 1, b = 2}` em vez de `{"a": 1, "b": 2}`.
  - Se o `value` não puder ser interpretado como um valor TOML válido, ele será tratado como uma string. Isso significa que tanto `-c model="o3"` quanto `-c model=o3` são equivalentes.
- O arquivo de configuração `$CODEX_HOME/config.toml`, onde o valor do ambiente `CODEX_HOME` padrão é `~/.codex`. (Note que `CODEX_HOME` também será onde logs e outras informações relacionadas ao Codex serão armazenadas.)

Tanto a flag `--config` quanto o arquivo `config.toml` suportam as seguintes opções:

## model

O modelo que o Codex deve usar.

```toml
model = "o3"  # substitui o padrão "gpt-5"
```

## model_providers

Esta opção permite substituir e complementar o conjunto padrão de provedores de modelo incluídos no Codex. Este valor é um mapa onde a chave é o valor usado em `model_provider` para selecionar o provedor correspondente.

Por exemplo, se você quiser adicionar um provedor que use o modelo OpenAI 4o via a API de chat completions, você poderia adicionar a seguinte configuração:

```toml
# Lembre-se que em TOML, as chaves raiz devem ser listadas antes das tabelas.
model = "gpt-4o"
model_provider = "openai-chat-completions"

[model_providers.openai-chat-completions]
# Nome do provedor que será exibido na interface do Codex.
name = "OpenAI using Chat Completions"
# O caminho `/chat/completions` será adicionado a esta URL para fazer a requisição POST
# para as chat completions.
base_url = "https://api.openai.com/v1"
# Se `env_key` estiver definido, identifica uma variável de ambiente que deve estar
# configurada ao usar o Codex com este provedor. O valor da variável deve ser
# não vazio e será usado no cabeçalho HTTP `Bearer TOKEN` para a requisição POST.
env_key = "OPENAI_API_KEY"
# Valores válidos para wire_api são "chat" e "responses". O padrão é "chat" se omitido.
wire_api = "chat"
# Se necessário, parâmetros extras de consulta que precisam ser adicionados à URL.
# Veja o exemplo do Azure abaixo.
query_params = {}
```

Note que isso torna possível usar o Codex CLI com modelos não OpenAI, desde que usem uma API compatível com a API de chat completions da OpenAI. Por exemplo, você poderia definir o seguinte provedor para usar o Codex CLI com Ollama rodando localmente:

```toml
[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"
```

Ou um provedor de terceiros (usando uma variável de ambiente distinta para a chave da API):

```toml
[model_providers.mistral]
name = "Mistral"
base_url = "https://api.mistral.ai/v1"
env_key = "MISTRAL_API_KEY"
```

Note que o Azure requer que `api-version` seja passado como parâmetro de consulta, então certifique-se de especificá-lo como parte de `query_params` ao definir o provedor Azure:

```toml
[model_providers.azure]
name = "Azure"
# Certifique-se de definir o subdomínio apropriado para esta URL.
base_url = "https://YOUR_PROJECT_NAME.openai.azure.com/openai"
env_key = "AZURE_OPENAI_API_KEY"  # Ou "OPENAI_API_KEY", o que você usar.
query_params = { api-version = "2025-04-01-preview" }
```

Também é possível configurar um provedor para incluir cabeçalhos HTTP extras em uma requisição. Estes podem ser valores fixos (`http_headers`) ou valores lidos de variáveis de ambiente (`env_http_headers`):

```toml
[model_providers.example]
# name, base_url, ...

# Isso adicionará o cabeçalho HTTP `X-Example-Header` com valor `example-value`
# a cada requisição para o provedor de modelo.
http_headers = { "X-Example-Header" = "example-value" }

# Isso adicionará o cabeçalho HTTP `X-Example-Features` com o valor da variável
# de ambiente `EXAMPLE_FEATURES` a cada requisição para o provedor de modelo
# _se_ a variável de ambiente estiver definida e seu valor não for vazio.
env_http_headers = { "X-Example-Features" = "EXAMPLE_FEATURES" }
```

### Ajustes de rede por provedor

As seguintes configurações opcionais controlam o comportamento de tentativas e timeouts de inatividade em streaming **por provedor de modelo**. Devem ser especificadas dentro do bloco correspondente `[model_providers.<id>]` em `config.toml`. (Versões antigas aceitavam chaves no nível superior; estas agora são ignoradas.)

Exemplo:

```toml
[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
# substituições de ajuste de rede (todas opcionais; usam padrões internos se omitidas)
request_max_retries = 4            # tenta novamente requisições HTTP falhadas
stream_max_retries = 10            # tenta reconectar streams SSE caídos
stream_idle_timeout_ms = 300000    # timeout de inatividade de 5 minutos
```

#### request_max_retries

Número de vezes que o Codex tentará repetir uma requisição HTTP falhada para o provedor de modelo. O padrão é `4`.

#### stream_max_retries

Número de vezes que o Codex tentará reconectar quando uma resposta em streaming for interrompida. O padrão é `5`.

#### stream_idle_timeout_ms

Tempo que o Codex aguardará por atividade em uma resposta em streaming antes de considerar a conexão perdida. O padrão é `300_000` (5 minutos).

## model_provider

Identifica qual provedor usar a partir do mapa `model_providers`. O padrão é `"openai"`. Você pode substituir o `base_url` para o provedor `openai` embutido via a variável de ambiente `OPENAI_BASE_URL`.

Note que se você substituir `model_provider`, provavelmente também desejará substituir
`model`. Por exemplo, se estiver executando ollama com Mistral localmente,
você precisará adicionar o seguinte à sua configuração além da nova entrada no mapa `model_providers`:

```toml
model_provider = "ollama"
model = "mistral"
```

## approval_policy

Determina quando o usuário deve ser solicitado a aprovar se o Codex pode executar um comando:

```toml
# O Codex tem uma lógica embutida que define um conjunto de comandos "confiáveis".
# Definir approval_policy como `untrusted` significa que o Codex solicitará
# aprovação do usuário antes de executar um comando que não esteja no conjunto "confiável".
#
# Veja https://github.com/openai/codex/issues/1260 para o plano de permitir que
# usuários finais definam seus próprios comandos confiáveis.
approval_policy = "untrusted"
```

Se você quiser ser notificado sempre que um comando falhar, use "on-failure":

```toml
# Se o comando falhar quando executado na sandbox, o Codex pede permissão para
# tentar executar o comando fora da sandbox.
approval_policy = "on-failure"
```

Se quiser que o modelo execute até decidir que precisa pedir permissões escaladas, use "on-request":

```toml
# O modelo decide quando escalar
approval_policy = "on-request"
```

Alternativamente, você pode fazer o modelo executar até o fim, sem nunca pedir para executar um comando com permissões escaladas:

```toml
# O usuário nunca é solicitado: se o comando falhar, o Codex tentará automaticamente
# outra coisa. Note que o subcomando `exec` sempre usa este modo.
approval_policy = "never"
```

## profiles

Um _profile_ é uma coleção de valores de configuração que podem ser definidos juntos. Múltiplos profiles podem ser definidos em `config.toml` e você pode especificar qual deseja usar em tempo de execução via a flag `--profile`.

Aqui está um exemplo de um `config.toml` que define múltiplos profiles:

```toml
model = "o3"
approval_policy = "untrusted"

# Definir `profile` é equivalente a especificar `--profile o3` na linha de comando,
# embora a flag `--profile` ainda possa ser usada para sobrescrever este valor.
profile = "o3"

[model_providers.openai-chat-completions]
name = "OpenAI using Chat Completions"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"

[profiles.o3]
model = "o3"
model_provider = "openai"
approval_policy = "never"
model_reasoning_effort = "high"
model_reasoning_summary = "detailed"

[profiles.gpt3]
model = "gpt-3.5-turbo"
model_provider = "openai-chat-completions"

[profiles.zdr]
model = "o3"
model_provider = "openai"
approval_policy = "on-failure"
```

Os usuários podem especificar valores de configuração em múltiplos níveis. A ordem de precedência é a seguinte:

1. argumento personalizado na linha de comando, por exemplo, `--model o3`
2. como parte de um profile, onde o `--profile` é especificado via CLI (ou no próprio arquivo de configuração)
3. como uma entrada em `config.toml`, por exemplo, `model = "o3"`
4. o valor padrão que vem com o Codex CLI (i.e., o padrão do Codex CLI é `gpt-5`)

## model_reasoning_effort

Se o modelo selecionado suporta raciocínio (por exemplo: `o3`, `o4-mini`, `codex-*`, `gpt-5`), o raciocínio é ativado por padrão ao usar a Responses API. Conforme explicado na [documentação da plataforma OpenAI](https://platform.openai.com/docs/guides/reasoning?api-mode=responses#get-started-with-reasoning), isso pode ser configurado para:

- `"minimal"`
- `"low"`
- `"medium"` (padrão)
- `"high"`

Nota: para minimizar o raciocínio, escolha `"minimal"`.

## model_reasoning_summary

Se o nome do modelo começa com `"o"` (como em `"o3"` ou `"o4-mini"`) ou `"codex"`, o raciocínio é ativado por padrão ao usar a Responses API. Conforme explicado na [documentação da plataforma OpenAI](https://platform.openai.com/docs/guides/reasoning?api-mode=responses#reasoning-summaries), isso pode ser configurado para:

- `"auto"` (padrão)
- `"concise"`
- `"detailed"`

Para desativar os resumos de raciocínio, defina `model_reasoning_summary` como `"none"` na sua configuração:

```toml
model_reasoning_summary = "none"  # desativa os resumos de raciocínio
```

## model_verbosity

Controla o comprimento/detalhe da saída nos modelos da família GPT‑5 ao usar a Responses API. Valores suportados:

- `"low"`
- `"medium"` (padrão quando omitido)
- `"high"`

Quando configurado, o Codex inclui um objeto `text` no payload da requisição com a verbosidade configurada, por exemplo: `"text": { "verbosity": "low" }`.

Exemplo:

```toml
model = "gpt-5"
model_verbosity = "low"
```

Nota: Isso se aplica apenas a provedores que usam a Responses API. Provedores de Chat Completions não são afetados.

## model_supports_reasoning_summaries

Por padrão, `reasoning` é definido apenas em requisições para modelos OpenAI que são conhecidos por suportá-lo. Para forçar `reasoning` a ser definido nas requisições para o modelo atual, você pode forçar esse comportamento definindo o seguinte em `config.toml`:

```toml
model_supports_reasoning_summaries = true
```

## sandbox_mode

O Codex executa comandos shell gerados pelo modelo dentro de uma sandbox em nível de sistema operacional.

Na maioria dos casos, você pode escolher o comportamento desejado com uma única opção:

```toml
# igual a `--sandbox read-only`
sandbox_mode = "read-only"
```

A política padrão é `read-only`, o que significa que comandos podem ler qualquer arquivo no disco, mas tentativas de escrever um arquivo ou acessar a rede serão bloqueadas.

Uma política mais relaxada é `workspace-write`. Quando especificada, o diretório de trabalho atual para a tarefa Codex será gravável (assim como `$TMPDIR` no macOS). Note que o CLI usa por padrão o diretório onde foi iniciado como `cwd`, embora isso possa ser sobrescrito usando `--cwd/-C`.

No macOS (e em breve no Linux), todas as raízes graváveis (incluindo `cwd`) que contenham uma pasta `.git/` _como filho imediato_ configurarão a pasta `.git/` como somente leitura enquanto o restante do repositório Git será gravável. Isso significa que comandos como `git commit` falharão, por padrão (pois envolvem escrita em `.git/`), e exigirã que o Codex peça permissão.

```toml
# igual a `--sandbox workspace-write`
sandbox_mode = "workspace-write"

# Configurações extras que se aplicam apenas quando `sandbox = "workspace-write"`.
[sandbox_workspace_write]
# Por padrão, o cwd para a sessão Codex será gravável, assim como $TMPDIR
# (se definido) e /tmp (se existir). Definir as opções respectivas como `true`
# sobrescreverá esses padrões.
exclude_tmpdir_env_var = false
exclude_slash_tmp = false

# Lista opcional de raízes graváveis _adicionais_ além de $TMPDIR e /tmp.
writable_roots = ["/Users/YOU/.pyenv/shims"]

# Permite que o comando executado dentro da sandbox faça requisições de rede externas.
# Desabilitado por padrão.
network_access = false
```

Para desabilitar a sandbox completamente, especifique `danger-full-access` assim:

```toml
# igual a `--sandbox danger-full-access`
sandbox_mode = "danger-full-access"
```

Isso é razoável de usar se o Codex estiver rodando em um ambiente que já fornece sua própria sandbox (como um container Docker), de modo que uma sandbox adicional não seja necessária.

Embora usar essa opção também possa ser necessário se você tentar usar o Codex em ambientes onde seus mecanismos nativos de sandbox não são suportados, como kernels Linux antigos ou no Windows.

## Approval presets

O Codex fornece três principais presets de aprovação:

- Read Only: Codex pode ler arquivos e responder perguntas; edições, execução de comandos e acesso à rede requerem aprovação.
- Auto: Codex pode ler arquivos, fazer edições e executar comandos no workspace sem aprovação; pede aprovação fora do workspace ou para acesso à rede.
- Full Access: Acesso total ao disco e rede sem prompts; extremamente arriscado.

Você pode personalizar ainda mais como o Codex roda na linha de comando usando as opções `--ask-for-approval` e `--sandbox`.

## mcp_servers

Define a lista de servidores MCP que o Codex pode consultar para uso de ferramentas. Atualmente, apenas servidores que são iniciados executando um programa que se comunica via stdio são suportados. Para servidores que usam transporte SSE, considere um adaptador como [mcp-proxy](https://github.com/sparfenyuk/mcp-proxy).

**Nota:** O Codex pode armazenar em cache a lista de ferramentas e recursos de um servidor MCP para incluir essa informação no contexto na inicialização sem precisar iniciar todos os servidores. Isso é projetado para economizar recursos carregando servidores MCP de forma preguiçosa.

Cada servidor pode definir `startup_timeout_ms` para ajustar quanto tempo o Codex espera para que ele inicie e responda a uma listagem de ferramentas. O padrão é `10_000` (10 segundos).

Esta opção de configuração é comparável a como Claude e Cursor definem `mcpServers` em seus respectivos arquivos JSON de configuração, embora, como o Codex usa TOML para sua linguagem de configuração, o formato seja ligeiramente diferente. Por exemplo, a seguinte configuração em JSON:

```json
{
  "mcpServers": {
    "server-name": {
      "command": "npx",
      "args": ["-y", "mcp-server"],
      "env": {
        "API_KEY": "value"
      }
    }
  }
}
```

Deve ser representada da seguinte forma em `~/.codex/config.toml`:

```toml
# IMPORTANTE: a chave de nível superior é `mcp_servers` em vez de `mcpServers`.
[mcp_servers.server-name]
command = "npx"
args = ["-y", "mcp-server"]
env = { "API_KEY" = "value" }
# Opcional: sobrescreve o timeout padrão de inicialização de 10s
startup_timeout_ms = 20_000
```

## shell_environment_policy

O Codex inicia subprocessos (por exemplo, ao executar uma chamada de ferramenta `local_shell` sugerida pelo assistente). Por padrão, ele agora passa **todo o seu ambiente** para esses subprocessos. Você pode ajustar esse comportamento via o bloco **`shell_environment_policy`** em `config.toml`:

```toml
[shell_environment_policy]
# inherit pode ser "all" (padrão), "core" ou "none"
inherit = "core"
# defina como true para *ignorar* o filtro para `"*KEY*"` e `"*TOKEN*"`
ignore_default_excludes = false
# padrões de exclusão (globs case-insensitive)
exclude = ["AWS_*", "AZURE_*"]
# força definir / sobrescrever valores
set = { CI = "1" }
# se fornecido, *apenas* variáveis que correspondem a esses padrões são mantidas
include_only = ["PATH", "HOME"]
```

| Campo                     | Tipo                       | Padrão | Descrição                                                                                                                                     |
| ------------------------- | -------------------------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `inherit`                 | string                     | `all`  | Template inicial para o ambiente:<br>`all` (clona o ambiente pai completo), `core` (`HOME`, `PATH`, `USER`, …), ou `none` (começa vazio).     |
| `ignore_default_excludes` | boolean                    | `false`| Quando `false`, o Codex remove qualquer variável cujo **nome** contenha `KEY`, `SECRET` ou `TOKEN` (case-insensitive) antes de outras regras.|
| `exclude`                 | array<string>              | `[]`   | Padrões glob case-insensitive para exclusão após o filtro padrão.<br>Exemplos: `"AWS_*"`, `"AZURE_*"`.                                        |
| `set`                     | table<string,string>       | `{}`   | Sobrescritas ou adições explícitas de chave/valor – sempre prevalecem sobre valores herdados.                                                |
| `include_only`            | array<string>              | `[]`   | Se não vazio, uma lista branca de padrões; apenas variáveis que correspondem a _um_ padrão sobrevivem à etapa final. (Geralmente usado com `inherit = "all"`.) |

Os padrões são no estilo **glob**, não expressões regulares completas: `*` corresponde a qualquer número de caracteres, `?` corresponde exatamente a um, e classes de caracteres como `[A-Z]`/`[^0-9]` são suportadas. A correspondência é sempre **case-insensitive**. Esta sintaxe está documentada no código como `EnvironmentVariablePattern` (veja `core/src/config_types.rs`).

Se você só precisa de um ambiente limpo com algumas entradas personalizadas, pode escrever:

```toml
[shell_environment_policy]
inherit = "none"
set = { PATH = "/usr/bin", MY_FLAG = "1" }
```

Atualmente, `CODEX_SANDBOX_NETWORK_DISABLED=1` também é adicionado ao ambiente, assumindo que a rede está desabilitada. Isso não é configurável.

## notify

Especifica um programa que será executado para receber notificações sobre eventos gerados pelo Codex. Note que o programa receberá o argumento de notificação como uma string JSON, por exemplo:

```json
{
  "type": "agent-turn-complete",
  "turn-id": "12345",
  "input-messages": ["Rename `foo` to `bar` and update the callsites."],
  "last-assistant-message": "Rename complete and verified `cargo build` succeeds."
}
```

A propriedade `"type"` sempre estará definida. Atualmente, `"agent-turn-complete"` é o único tipo de notificação suportado.

Como exemplo, aqui está um script Python que analisa o JSON e decide se deve mostrar uma notificação push na área de trabalho usando [terminal-notifier](https://github.com/julienXX/terminal-notifier) no macOS:

```python
#!/usr/bin/env python3

import json
import subprocess
import sys


def main() -> int:
    if len(sys.argv) != 2:
        print("Usage: notify.py <NOTIFICATION_JSON>")
        return 1

    try:
        notification = json.loads(sys.argv[1])
    except json.JSONDecodeError:
        return 1

    match notification_type := notification.get("type"):
        case "agent-turn-complete":
            assistant_message = notification.get("last-assistant-message")
            if assistant_message:
                title = f"Codex: {assistant_message}"
            else:
                title = "Codex: Turn Complete!"
            input_messages = notification.get("input_messages", [])
            message = " ".join(input_messages)
            title += message
        case _:
            print(f"not sending a push notification for: {notification_type}")
            return 0

    subprocess.check_output(
        [
            "terminal-notifier",
            "-title",
            title,
            "-message",
            message,
            "-group",
            "codex",
            "-ignoreDnD",
            "-activate",
            "com.googlecode.iterm2",
        ]
    )

    return 0


if __name__ == "__main__":
    sys.exit(main())
```

Para que o Codex use este script para notificações, você deve configurá-lo via `notify` em `~/.codex/config.toml` usando o caminho apropriado para `notify.py` no seu computador:

```toml
notify = ["python3", "/Users/mbolin/.codex/notify.py"]
```

## history

Por padrão, o Codex CLI registra mensagens enviadas ao modelo em `$CODEX_HOME/history.jsonl`. Note que em sistemas UNIX, as permissões do arquivo são definidas para `o600`, então ele deve ser legível e gravável apenas pelo proprietário.

Para desabilitar esse comportamento, configure `[history]` da seguinte forma:

```toml
[history]
persistence = "none"  # "save-all" é o valor padrão
```

## file_opener

Identifica o editor/esquema URI a ser usado para hyperlinkar citações na saída do modelo. Se definido, citações para arquivos na saída do modelo serão hyperlinkadas usando o esquema URI especificado para que possam ser clicadas com ctrl/cmd no terminal para abrir os arquivos.

Por exemplo, se a saída do modelo incluir uma referência como `【F:/home/user/project/main.py†L42-L50】`, isso será reescrito para linkar para o URI `vscode://file/home/user/project/main.py:42`.

Note que isso **não** é uma configuração geral de editor (como `$EDITOR`), pois aceita apenas um conjunto fixo de valores:

- `"vscode"` (padrão)
- `"vscode-insiders"`
- `"windsurf"`
- `"cursor"`
- `"none"` para desabilitar explicitamente este recurso

Atualmente, `"vscode"` é o padrão, embora o Codex não verifique se o VS Code está instalado. Assim, `file_opener` pode ter o padrão `"none"` ou outro valor no futuro.

## hide_agent_reasoning

O Codex emite intermitentemente eventos de "raciocínio" que mostram o "pensamento" interno do modelo antes de produzir uma resposta final. Alguns usuários podem achar esses eventos distrativos, especialmente em logs de CI ou saída mínima no terminal.

Definir `hide_agent_reasoning` como `true` suprime esses eventos **tanto** na TUI quanto no subcomando headless `exec`:

```toml
hide_agent_reasoning = true   # padrão é false
```

## show_raw_agent_reasoning

Exibe o conteúdo bruto da cadeia de raciocínio do modelo ("raw reasoning content") quando disponível.

Notas:

- Só tem efeito se o modelo/provedor selecionado realmente emitir conteúdo bruto de raciocínio. Muitos modelos não o fazem. Quando não suportado, esta opção não tem efeito visível.
- O raciocínio bruto pode incluir pensamentos intermediários ou contexto sensível. Ative apenas se for aceitável para seu fluxo de trabalho.

Exemplo:

```toml
show_raw_agent_reasoning = true  # padrão é false
```

## model_context_window

O tamanho da janela de contexto para o modelo, em tokens.

Em geral, o Codex conhece a janela de contexto para os modelos OpenAI mais comuns, mas se você estiver usando um modelo novo com uma versão antiga do Codex CLI, pode usar `model_context_window` para informar ao Codex qual valor usar para determinar quanto contexto resta durante uma conversa.

## model_max_output_tokens

Análogo a `model_context_window`, mas para o número máximo de tokens de saída do modelo.

## project_doc_max_bytes

Número máximo de bytes a serem lidos de um arquivo `AGENTS.md` para incluir nas instruções enviadas com o primeiro turno de uma sessão. O padrão é 32 KiB.

## tui

Opções específicas para a TUI.

```toml
[tui]
# Mais opções virão aqui
```

## Referência de configuração

| Chave                                  | Tipo / Valores                              | Notas                                                                                  |
|---------------------------------------|--------------------------------------------|----------------------------------------------------------------------------------------|
| `model`                               | string                                     | Modelo a ser usado (ex: `gpt-5`).                                                     |
| `model_provider`                      | string                                     | ID do provedor do mapa `model_providers` (padrão: `openai`).                          |
| `model_context_window`                | number                                     | Tokens da janela de contexto.                                                         |
| `model_max_output_tokens`             | number                                     | Máximo de tokens de saída.                                                            |
| `approval_policy`                     | `untrusted` \| `on-failure` \| `on-request` \| `never` | Quando solicitar aprovação.                                                            |
| `sandbox_mode`                        | `read-only` \| `workspace-write` \| `danger-full-access` | Política da sandbox do SO.                                                             |
| `sandbox_workspace_write.writable_roots` | array<string>                              | Raízes graváveis extras em workspace-write.                                           |
| `sandbox_workspace_write.network_access` | boolean                                   | Permitir rede em workspace-write (padrão: false).                                     |
| `sandbox_workspace_write.exclude_tmpdir_env_var` | boolean                              | Excluir `$TMPDIR` das raízes graváveis (padrão: false).                               |
| `sandbox_workspace_write.exclude_slash_tmp` | boolean                                  | Excluir `/tmp` das raízes graváveis (padrão: false).                                  |
| `disable_response_storage`            | boolean                                    | Necessário para organizações ZDR.                                                     |
| `notify`                             | array<string>                              | Programa externo para notificações.                                                   |
| `instructions`                      | string                                     | Atualmente ignorado; use `experimental_instructions_file` ou `AGENTS.md`.             |
| `mcp_servers.<id>.command`           | string                                     | Comando para iniciar servidor MCP.                                                   |
| `mcp_servers.<id>.args`              | array<string>                              | Argumentos para servidor MCP.                                                         |
| `mcp_servers.<id>.env`               | map<string,string>                         | Variáveis de ambiente para servidor MCP.                                             |
| `mcp_servers.<id>.startup_timeout_ms` | number                                   | Timeout de inicialização em ms (padrão: 10_000). Timeout aplicado para inicializar e listar ferramentas. |
| `model_providers.<id>.name`          | string                                     | Nome exibido.                                                                         |
| `model_providers.<id>.base_url`      | string                                     | URL base da API.                                                                      |
| `model_providers.<id>.env_key`       | string                                     | Variável de ambiente para chave da API.                                              |
| `model_providers.<id>.wire_api`      | `chat` \| `responses`                      | Protocolo usado (padrão: `chat`).                                                    |
| `model_providers.<id>.query_params`  | map<string,string>                         | Parâmetros extras de consulta (ex: `api-version` do Azure).                           |
| `model_providers.<id>.http_headers`  | map<string,string>                         | Cabeçalhos estáticos adicionais.                                                     |
| `model_providers.<id>.env_http_headers` | map<string,string>                      | Cabeçalhos obtidos de variáveis de ambiente.                                         |
| `model_providers.<id>.request_max_retries` | number                                | Número de tentativas HTTP por provedor (padrão: 4).                                  |
| `model_providers.<id>.stream_max_retries` | number                                 | Número de tentativas para streams SSE (padrão: 5).                                   |
| `model_providers.<id>.stream_idle_timeout_ms` | number                              | Timeout de inatividade SSE em ms (padrão: 300000).                                   |
| `project_doc_max_bytes`              | number                                     | Máximo de bytes para ler de `AGENTS.md`.                                             |
| `profile`                           | string                                     | Nome do profile ativo.                                                                |
| `profiles.<name>.*`                 | vários                                     | Sobrescritas de keys no escopo do profile.                                           |
| `history.persistence`               | `save-all` \| `none`                       | Persistência do arquivo de histórico (padrão: `save-all`).                            |
| `history.max_bytes`                 | number                                     | Atualmente ignorado (não aplicado).                                                  |
| `file_opener`                      | `vscode` \| `vscode-insiders` \| `windsurf` \| `cursor` \| `none` | Esquema URI para citações clicáveis (padrão: `vscode`).                              |
| `tui`                             | table                                      | Opções específicas da TUI (reservadas).                                              |
| `hide_agent_reasoning`             | boolean                                    | Oculta eventos de raciocínio do modelo.                                              |
| `show_raw_agent_reasoning`         | boolean                                    | Exibe raciocínio bruto (quando disponível).                                          |
| `model_reasoning_effort`           | `minimal` \| `low` \| `medium` \| `high` | Esforço de raciocínio na Responses API.                                              |
| `model_reasoning_summary`          | `auto` \| `concise` \| `detailed` \| `none` | Resumos de raciocínio.                                                               |
| `model_verbosity`                  | `low` \| `medium` \| `high`                | Verbosidade do texto GPT‑5 (Responses API).                                          |
| `model_supports_reasoning_summaries` | boolean                                  | Força habilitar resumos de raciocínio.                                               |
| `model_reasoning_summary_format`  | `none` \| `experimental`                    | Força formato do resumo de raciocínio.                                               |
| `chatgpt_base_url`                 | string                                     | URL base para fluxo de autenticação ChatGPT.                                         |
| `experimental_resume`              | string (caminho)                           | Caminho JSONL de resumo (interno/experimental).                                      |
| `experimental_instructions_file`  | string (caminho)                           | Substitui instruções embutidas (experimental).                                       |
| `experimental_use_exec_command_tool` | boolean                                  | Usa ferramenta experimental de comando exec.                                         |
| `responses_originator_header_internal_override` | string                              | Sobrescreve valor do cabeçalho `originator`.                                         |
| `projects.<path>.trust_level`      | string                                     | Marca projeto/worktree como confiável (apenas `"trusted"` é reconhecido).            |
| `preferred_auth_method`            | `chatgpt` \| `apikey`                      | Seleciona método padrão de autenticação (padrão: `chatgpt`).                         |
| `tools.web_search`                 | boolean                                    | Habilita ferramenta de busca web (alias: `web_search_request`) (padrão: false).      |