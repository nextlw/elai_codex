## Prompts Customizados

Salve prompts usados com frequência como arquivos Markdown e reutilize-os rapidamente pelo menu de barra (/).

- Localização: Coloque os arquivos em `$CODEX_HOME/prompts/` (padrão `~/.codex/prompts/`).
- Tipo de arquivo: Apenas arquivos Markdown com extensão `.md` são reconhecidos.
- Nome: O nome do arquivo sem a extensão `.md` vira a entrada da barra. Por exemplo, para um arquivo chamado `my-prompt.md`, digite `/my-prompt`.
- Conteúdo: O conteúdo do arquivo é enviado como sua mensagem quando você seleciona o item no popup da barra e pressiona Enter.
- Como usar:
  - Inicie uma nova sessão (o Codex carrega os prompts customizados ao iniciar a sessão).
  - No compositor, digite `/` para abrir o popup da barra e comece a digitar o nome do prompt.
  - Use as setas para cima/baixo para selecionar. Pressione Enter para enviar o conteúdo ou Tab para autocompletar o nome.
- Notas:
  - Arquivos com nomes que colidam com comandos internos (ex: `/init`) são ignorados e não aparecem.
  - Arquivos novos ou alterados são detectados ao iniciar a sessão. Se adicionar um prompt enquanto o Codex estiver rodando, inicie uma nova sessão para que ele seja carregado.