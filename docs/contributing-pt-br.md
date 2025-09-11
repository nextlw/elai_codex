## Contribuindo

Este projeto está em desenvolvimento ativo e o código provavelmente mudará significativamente.

**No momento, planejamos priorizar a revisão de contribuições externas apenas para correções de bugs ou segurança.**

Se você quiser adicionar um novo recurso ou alterar o comportamento de um existente, por favor, abra uma issue propondo o recurso e obtenha aprovação de um membro da equipe OpenAI antes de investir tempo no desenvolvimento.

**Novas contribuições que não seguirem esse processo podem ser fechadas** se não estiverem alinhadas com nosso roadmap atual ou conflitarem com outras prioridades/funcionalidades futuras.

### Fluxo de desenvolvimento

- Crie uma _branch de tópico_ a partir da `main` - ex: `feat/interactive-prompt`.
- Mantenha suas mudanças focadas. Correções não relacionadas devem ser abertas como PRs separados.
- Seguindo as instruções de [configuração do desenvolvimento](#development-workflow), garanta que sua alteração não tenha avisos de lint nem falhas em testes.

### Escrevendo mudanças de código de alto impacto

1. **Comece com uma issue.** Abra uma nova ou comente em uma discussão existente para que possamos concordar na solução antes de escrever código.
2. **Adicione ou atualize testes.** Toda nova funcionalidade ou correção deve ter cobertura de testes que falha antes da sua mudança e passa depois. Cobertura 100% não é obrigatória, mas busque asserções significativas.
3. **Documente o comportamento.** Se sua mudança afetar o comportamento visível ao usuário, atualize o README, a ajuda inline (`codex --help`) ou projetos de exemplo relevantes.
4. **Mantenha commits atômicos.** Cada commit deve compilar e passar nos testes. Isso facilita revisões e possíveis reversões.

### Abrindo um pull request

- Preencha o template do PR (ou inclua informações similares) - **O quê? Por quê? Como?**
- Execute **todos** os checks localmente (`cargo test && cargo clippy --tests && cargo fmt -- --config imports_granularity=Item`). Falhas no CI que poderiam ser detectadas localmente atrasam o processo.
- Garanta que sua branch esteja atualizada com a `main` e que conflitos de merge estejam resolvidos.
- Marque o PR como **Pronto para revisão** apenas quando acreditar que está em estado mergeável.

### Processo de revisão

1. Um mantenedor será designado como revisor principal.
2. Se seu PR adicionar um recurso novo que não foi discutido e aprovado previamente, podemos optar por fechá-lo (veja [Contributing](#contributing)).
3. Podemos solicitar mudanças - por favor, não leve para o lado pessoal. Valorizamos o trabalho, mas também a consistência e a manutenção a longo prazo.
4. Quando houver consenso de que o PR atende aos critérios, um mantenedor fará squash-and-merge.

### Valores da comunidade

- **Seja gentil e inclusivo.** Trate os outros com respeito; seguimos o [Contributor Covenant](https://www.contributor-covenant.org/).
- **Presuma boa intenção.** Comunicação escrita é difícil - prefira a generosidade.
- **Ensine e aprenda.** Se algo parecer confuso, abra uma issue ou PR com melhorias.

### Obter ajuda

Se tiver problemas para configurar o projeto, quiser feedback sobre uma ideia ou apenas dizer _oi_, abra uma Discussion ou participe da issue relevante. Estamos felizes em ajudar.

Juntos podemos fazer do Codex CLI uma ferramenta incrível. **Boas codificações!** :rocket:

### Acordo de licença do contribuidor (CLA)

Todos os contribuintes **devem** aceitar o CLA. O processo é simples:

1. Abra seu pull request.
2. Cole o seguinte comentário (ou responda `recheck` se já tiver assinado antes):

   ```text
   I have read the CLA Document and I hereby sign the CLA
   ```

3. O bot CLA-Assistant registra sua assinatura no repositório e marca o status check como aprovado.

Nenhum comando Git especial, anexo de e-mail ou rodapé de commit é necessário.

#### Correções rápidas

| Cenário           | Comando                                           |
| ----------------- | ------------------------------------------------ |
| Corrigir último commit | `git commit --amend -s --no-edit && git push -f` |

O **check DCO** bloqueia merges até que todo commit no PR tenha o rodapé (com squash, é só o único).

### Liberando o `codex`

_Apenas para administradores._

Garanta que está na branch `main` e que não há mudanças locais. Então execute:

```shell
VERSION=0.2.0  # Pode ser também 0.2.0-alpha.1 ou qualquer versão válida do Rust.
./codex-rs/scripts/create_github_release.sh "$VERSION"
```

Isso fará um commit local no topo da `main` com a `version` definida para `$VERSION` no `codex-rs/Cargo.toml` (note que na `main` deixamos a versão como `version = "0.0.0"`).

Isso enviará o commit usando a tag `rust-v${VERSION}`, que por sua vez dispara o [workflow de release](../.github/workflows/rust-release.yml). Isso criará um novo GitHub Release nomeado `$VERSION`.

Se tudo parecer correto no GitHub Release gerado, desmarque a caixa de **pre-release** para que seja o release mais recente.

Crie um PR para atualizar [`Formula/c/codex.rb`](https://github.com/Homebrew/homebrew-core/blob/main/Formula/c/codex.rb) no Homebrew.

### Segurança & IA responsável

Descobriu uma vulnerabilidade ou tem preocupações sobre a saída do modelo? Por favor, envie um e-mail para **security@openai.com** e responderemos prontamente.