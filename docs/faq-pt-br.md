## FAQ

### A OpenAI lançou um modelo chamado Codex em 2021 - isso está relacionado?

Em 2021, a OpenAI lançou o Codex, um sistema de IA projetado para gerar código a partir de comandos em linguagem natural. Esse modelo original do Codex foi descontinuado em março de 2023 e é separado da ferramenta CLI.

### Quais modelos são suportados?

Recomendamos usar o Codex com o GPT-5, nosso melhor modelo para codificação. O nível padrão de raciocínio é médio, e você pode aumentar para alto para tarefas complexas com o comando `/model`.

Você também pode usar modelos mais antigos utilizando autenticação baseada em API e iniciando o codex com a flag `--model`.

### Por que `o3` ou `o4-mini` não funcionam para mim?

É possível que sua [conta de API precise ser verificada](https://help.openai.com/en/articles/10910291-api-organization-verification) para começar a receber respostas em streaming e visualizar resumos de cadeia de raciocínio da API. Se continuar enfrentando problemas, por favor, nos informe!

### Como faço para impedir que o Codex edite meus arquivos?

Por padrão, o Codex pode modificar arquivos no seu diretório de trabalho atual (modo Auto). Para evitar edições, execute `codex` em modo somente leitura com a flag CLI `--sandbox read-only`. Alternativamente, você pode alterar o nível de aprovação durante a conversa com `/approvals`.

### Funciona no Windows?

Executar o Codex diretamente no Windows pode funcionar, mas não é oficialmente suportado. Recomendamos usar o [Windows Subsystem for Linux (WSL2)](https://learn.microsoft.com/en-us/windows/wsl/install).