## Sandbox & aprovações

### Modos de aprovação

Escolhemos um padrão poderoso para como o Codex funciona no seu computador: `Auto`. Neste modo de aprovação, o Codex pode ler arquivos, fazer edições e executar comandos no diretório de trabalho automaticamente. Contudo, o Codex precisará da sua aprovação para trabalhar fora do diretório de trabalho ou acessar a rede.

Quando você só quer conversar, ou se deseja planejar antes de começar, pode mudar para o modo `Somente leitura` com o comando `/approvals`.

Se precisar que o Codex leia arquivos, faça edições e execute comandos com acesso à rede, sem aprovação, pode usar o modo `Acesso total`. Use com cautela.

#### Padrões e recomendações

- O Codex roda em uma sandbox por padrão com fortes proteções: ele impede editar arquivos fora do workspace e bloqueia acesso à rede, a menos que habilitado.
- Ao iniciar, o Codex detecta se a pasta é versionada e recomenda:
  - Pastas versionadas: `Auto` (escrita no workspace + aprovações sob demanda)
  - Pastas não versionadas: `Somente leitura`
- O workspace inclui o diretório atual e diretórios temporários como `/tmp`. Use o comando `/status` para ver quais diretórios estão no workspace.
- Você pode configurar explicitamente:
  - `codex --sandbox workspace-write --ask-for-approval on-request`
  - `codex --sandbox read-only --ask-for-approval on-request`

### Posso rodar sem NENHUMA aprovação?

Sim, você pode desabilitar todos os prompts de aprovação com `--ask-for-approval never`. Essa opção funciona com todos os modos `--sandbox`, então você ainda mantém controle total sobre o nível de autonomia do Codex. Ele fará o melhor possível com as restrições que você fornecer.

### Combinações comuns de sandbox + aprovações

| Intenção                               | Flags                                                                                   | Efeito                                                                                   |
| ------------------------------------- | --------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Navegação segura somente leitura      | `--sandbox read-only --ask-for-approval on-request`                                    | Codex pode ler arquivos e responder perguntas. Requer aprovação para editar, executar comandos ou acessar rede. |
| Somente leitura não interativo (CI)  | `--sandbox read-only --ask-for-approval never`                                         | Apenas leitura; nunca eleva permissões                                                    |
| Permitir editar o repositório, pedir se arriscado | `--sandbox workspace-write --ask-for-approval on-request`                              | Codex pode ler, editar e executar comandos no workspace. Requer aprovação para ações fora do workspace ou acesso à rede. |
| Auto (preset)                        | `--full-auto` (equivalente a `--sandbox workspace-write` + `--ask-for-approval on-failure`) | Codex pode ler, editar e executar comandos no workspace. Requer aprovação quando um comando sandbox falha ou precisa de escalonamento. |
| YOLO (não recomendado)                | `--dangerously-bypass-approvals-and-sandbox` (alias: `--yolo`)                         | Sem sandbox; sem prompts                                                                 |

> Nota: No modo `workspace-write`, a rede está desabilitada por padrão, a menos que habilitada na configuração (`[sandbox_workspace_write].network_access = true`).

#### Ajustes finos em `config.toml`

```toml
# modo de aprovação
approval_policy = "untrusted"
sandbox_mode    = "read-only"

# modo full-auto
approval_policy = "on-request"
sandbox_mode    = "workspace-write"

# Opcional: permitir rede no modo workspace-write
[sandbox_workspace_write]
network_access = true
```

Você também pode salvar presets como **perfis**:

```toml
[profiles.full_auto]
approval_policy = "on-request"
sandbox_mode    = "workspace-write"

[profiles.readonly_quiet]
approval_policy = "never"
sandbox_mode    = "read-only"
```

### Experimentando com a Sandbox do Codex

Para testar o que acontece quando um comando é executado sob a sandbox provida pelo Codex, fornecemos os seguintes subcomandos no Codex CLI:

```
# macOS
codex debug seatbelt [--full-auto] [COMMAND]...

# Linux
codex debug landlock [--full-auto] [COMMAND]...
```

### Detalhes da sandbox por plataforma

O mecanismo que o Codex usa para implementar a política de sandbox depende do seu sistema operacional:

- **macOS 12+** usa **Apple Seatbelt** e executa comandos usando `sandbox-exec` com um perfil (`-p`) que corresponde ao `--sandbox` especificado.
- **Linux** usa uma combinação das APIs Landlock/seccomp para aplicar a configuração do `sandbox`.

Note que ao rodar Linux em ambiente containerizado como Docker, a sandbox pode não funcionar se a configuração do host/container não suportar as APIs necessárias Landlock/seccomp. Nestes casos, recomendamos configurar seu container Docker para prover as garantias de sandbox desejadas e então rodar `codex` com `--sandbox danger-full-access` (ou, mais simplesmente, a flag `--dangerously-bypass-approvals-and-sandbox`) dentro do container.