### Detalhes técnicos do sandbox da plataforma

O mecanismo que o Codex usa para implementar a política de sandbox depende do seu sistema operacional:

- **macOS 12+** utiliza o **Apple Seatbelt** e executa comandos usando `sandbox-exec` com um perfil (`-p`) que corresponde ao `--sandbox` especificado.
- **Linux** utiliza uma combinação das APIs Landlock/seccomp para aplicar a configuração do `sandbox`.

Note que, ao executar o Linux em um ambiente containerizado como Docker, o sandbox pode não funcionar se a configuração do host/container não suportar as APIs Landlock/seccomp necessárias. Nesses casos, recomendamos configurar seu container Docker para que ele forneça as garantias de sandbox desejadas e então executar o `codex` com `--sandbox danger-full-access` (ou, mais simplesmente, a flag `--dangerously-bypass-approvals-and-sandbox`) dentro do container.