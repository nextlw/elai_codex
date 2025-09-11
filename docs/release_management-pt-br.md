# Gerenciamento de Releases

Atualmente, disponibilizamos os binários do Codex em três locais:

- GitHub Releases https://github.com/openai/codex/releases/
- `@openai/codex` no npm: https://www.npmjs.com/package/@openai/codex
- `codex` no Homebrew: https://formulae.brew.sh/formula/codex

# Criando uma Release

Execute o script `codex-rs/scripts/create_github_release` no repositório para publicar uma nova release. O script escolherá o número de versão apropriado dependendo do tipo de release que você está criando.

Para criar uma nova release alpha a partir da branch `main` (sinta-se à vontade para criar alphas livremente):

```
./codex-rs/scripts/create_github_release --publish-alpha
```

Para criar uma nova release _pública_ a partir da branch `main` (o que requer mais cautela), execute:

```
./codex-rs/scripts/create_github_release --publish-release
```

DICA: Adicione a flag `--dry-run` para reportar o próximo número de versão para a release respectiva e sair.

Executar o script de publicação iniciará uma GitHub Action para construir a release, então acesse https://github.com/openai/codex/actions/workflows/rust-release.yml para encontrar o workflow correspondente. (Nota: deveríamos automatizar a busca da URL do workflow com `gh`.)

Quando o workflow terminar, a GitHub Release estará "concluída", mas você ainda precisa considerar o npm e o Homebrew.

## Publicando no npm

Após a GitHub Release estar concluída, você pode publicar no npm. Note que a GitHub Release inclui o artefato apropriado para o npm (que é a saída do `npm pack`), que deve ser nomeado `codex-npm-VERSION.tgz`. Para publicar no npm, execute:

```
VERSION=0.21.0
./scripts/publish_to_npm.py "$VERSION"
```

Observe que você deve ter permissões para publicar em https://www.npmjs.com/package/@openai/codex para que isso tenha sucesso.

## Publicando no Homebrew

Para o Homebrew, estamos devidamente configurados com o sistema de automação deles, então a cada poucas horas ele verifica nosso repositório no GitHub para ver se há uma nova release. Quando encontrar uma, ele criará um PR para criar a release equivalente no Homebrew, o que implica construir o Codex CLI a partir do código-fonte em várias versões do macOS.

Inevitavelmente, você só precisa atualizar esta página periodicamente para ver se a release foi capturada pelo sistema de automação deles:

https://github.com/Homebrew/homebrew-core/pulls?q=%3Apr+codex

Quando tudo for construído, um administrador do Homebrew precisa aprovar o PR. Novamente, o processo todo leva várias horas e não temos controle total sobre ele, mas parece funcionar muito bem.

Para referência, nossa fórmula do Homebrew está em:

https://github.com/Homebrew/homebrew-core/blob/main/Formula/c/codex.rb