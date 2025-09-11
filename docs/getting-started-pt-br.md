## Começando

### Uso da CLI

| Comando            | Propósito                            | Exemplo                         |
| ------------------ | ---------------------------------- | ------------------------------- |
| `codex`            | TUI interativa                     | `codex`                         |
| `codex "..."`      | Prompt inicial para TUI interativa | `codex "fix lint errors"`       |
| `codex exec "..."` | "modo de automação" não interativo | `codex exec "explain utils.ts"` |

Flags principais: `--model/-m`, `--ask-for-approval/-a`.

<!--
Opções de retomada:

- `--resume`: abre um seletor interativo de sessões recentes (mostra uma prévia da primeira mensagem real do usuário). Conflita com `--continue`.
- `--continue`: retoma a sessão mais recente sem mostrar o seletor (recomeça do zero se não houver sessões). Conflita com `--resume`.

Exemplos:

```shell
codex --resume
codex --continue
```
-->

### Executando com um prompt como entrada

Você também pode executar o Codex CLI com um prompt como entrada:

```shell
codex "explain this codebase to me"
```

```shell
codex --full-auto "create the fanciest todo-list app"
```

É isso - o Codex irá criar um arquivo, executá-lo dentro de uma sandbox, instalar quaisquer dependências faltantes e mostrar o resultado ao vivo. Aprove as mudanças e elas serão commitadas no seu diretório de trabalho.

### Exemplos de prompts

Abaixo estão alguns exemplos rápidos que você pode copiar e colar. Substitua o texto entre aspas pela sua própria tarefa. Veja o [guia de prompting](https://github.com/openai/codex/blob/main/codex-cli/examples/prompting_guide.md) para mais dicas e padrões de uso.

| ✨  | O que você digita                                                               | O que acontece                                                             |
| --- | ------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| 1   | `codex "Refactor the Dashboard component to React Hooks"`                       | O Codex reescreve o componente de classe, executa `npm test` e mostra o diff.   |
| 2   | `codex "Generate SQL migrations for adding a users table"`                      | Infere seu ORM, cria arquivos de migração e executa-os em um banco sandbox. |
| 3   | `codex "Write unit tests for utils/date.ts"`                                    | Gera testes, executa-os e itera até que passem.                            |
| 4   | `codex "Bulk-rename *.jpeg -> *.jpg with git mv"`                               | Renomeia arquivos com segurança e atualiza imports/usos.                   |
| 5   | `codex "Explain what this regex does: ^(?=.*[A-Z]).{8,}$"`                      | Gera uma explicação humana passo a passo.                                  |
| 6   | `codex "Carefully review this repo, and propose 3 high impact well-scoped PRs"` | Sugere PRs impactantes no código atual.                                    |
| 7   | `codex "Look for vulnerabilities and create a security review report"`          | Encontra e explica bugs de segurança.                                      |

### Memória com AGENTS.md

Você pode dar instruções e orientações extras ao Codex usando arquivos `AGENTS.md`. O Codex procura arquivos `AGENTS.md` nos seguintes locais e os mescla de cima para baixo:

1. `~/.codex/AGENTS.md` - orientação global pessoal
2. `AGENTS.md` na raiz do repositório - notas compartilhadas do projeto
3. `AGENTS.md` no diretório atual - especificidades de subpasta/feature

Para mais informações sobre como usar AGENTS.md, veja a [documentação oficial do AGENTS.md](https://agents.md/).

### Dicas & atalhos

#### Use `@` para busca de arquivos

Digitar `@` ativa uma busca fuzzy por nome de arquivo na raiz do workspace. Use as setas para cima/baixo para selecionar entre os resultados e Tab ou Enter para substituir o `@` pelo caminho selecionado. Use Esc para cancelar a busca.

#### Entrada de imagem

Cole imagens diretamente no compositor (Ctrl+V / Cmd+V) para anexá-las ao seu prompt. Você também pode anexar arquivos via CLI usando `-i/--image` (separados por vírgula):

```bash
codex -i screenshot.png "Explain this error"
codex --image img1.png,img2.jpg "Summarize these diagrams"
```

#### Esc–Esc para editar uma mensagem anterior

Quando o compositor de chat estiver vazio, pressione Esc para ativar o modo “backtrack”. Pressione Esc novamente para abrir uma prévia da transcrição destacando a última mensagem do usuário; pressione Esc repetidamente para navegar por mensagens anteriores. Pressione Enter para confirmar e o Codex irá bifurcar a conversa a partir daquele ponto, recortando a transcrição visível e pré-preenchendo o compositor com a mensagem selecionada para edição e reenvio.

Na prévia da transcrição, o rodapé mostra a dica `Esc edit prev` enquanto a edição está ativa.

#### Completações de shell

Gere scripts de completação para shell via:

```shell
codex completion bash
codex completion zsh
codex completion fish
```

#### Flag `--cd`/`-C`

Às vezes não é conveniente fazer `cd` para o diretório que você quer que o Codex use como "raiz de trabalho" antes de executá-lo. Felizmente, o `codex` suporta a opção `--cd` para que você possa especificar qualquer pasta que desejar. Você pode confirmar que o Codex está respeitando o `--cd` verificando o **workdir** que ele reporta na TUI no início de uma nova sessão.