# Autenticação

## Alternativa de cobrança por uso: Use uma chave API da OpenAI

Se preferir pagar conforme o uso, você ainda pode autenticar com sua chave API da OpenAI definindo-a como uma variável de ambiente:

```shell
export OPENAI_API_KEY="your-api-key-here"
```

Essa chave deve, no mínimo, ter permissão de escrita na API de Respostas.

## Migrando para login via ChatGPT a partir da chave API

Se você já usou o Codex CLI antes com cobrança por uso via chave API e deseja mudar para usar seu plano ChatGPT, siga estes passos:

1. Atualize a CLI e certifique-se que `codex --version` seja `0.20.0` ou superior
2. Delete `~/.codex/auth.json` (no Windows: `C:\\Users\\USERNAME\\.codex\\auth.json`)
3. Execute `codex login` novamente

## Forçando um método de autenticação específico (avançado)

Você pode escolher explicitamente qual autenticação o Codex deve preferir quando ambos estiverem disponíveis.

- Para sempre usar sua chave API (mesmo quando a autenticação ChatGPT existir), defina:

```toml
# ~/.codex/config.toml
preferred_auth_method = "apikey"
```

Ou sobrescreva ad-hoc via CLI:

```bash
codex --config preferred_auth_method="apikey"
```

- Para preferir autenticação ChatGPT (padrão), defina:

```toml
# ~/.codex/config.toml
preferred_auth_method = "chatgpt"
```

Notas:

- Quando `preferred_auth_method = "apikey"` e uma chave API estiver disponível, a tela de login é pulada.
- Quando `preferred_auth_method = "chatgpt"` (padrão), o Codex prefere autenticação ChatGPT se presente; se apenas uma chave API estiver disponível, usará a chave API. Certos tipos de conta também podem exigir o modo chave API.
- Para verificar qual método de autenticação está sendo usado durante uma sessão, use o comando `/status` na TUI.

## Conectando em uma máquina "headless"

Hoje, o processo de login envolve rodar um servidor em `localhost:1455`. Se você estiver em um servidor "headless", como um container Docker ou conectado via `ssh` em uma máquina remota, carregar `localhost:1455` no navegador da sua máquina local não conectará automaticamente ao servidor web rodando na máquina _headless_, então você deve usar uma das seguintes soluções:

### Autentique localmente e copie suas credenciais para a máquina "headless"

A solução mais fácil é executar o processo `codex login` na sua máquina local, de modo que `localhost:1455` _esteja_ acessível no seu navegador. Quando completar o processo de autenticação, um arquivo `auth.json` deve estar disponível em `$CODEX_HOME/auth.json` (no Mac/Linux, `$CODEX_HOME` padrão é `~/.codex`, no Windows é `%USERPROFILE%\\.codex`).

Como o arquivo `auth.json` não está vinculado a um host específico, uma vez que você completar o fluxo de autenticação localmente, pode copiar o arquivo `$CODEX_HOME/auth.json` para a máquina headless e então o `codex` deve funcionar normalmente nessa máquina. Para copiar um arquivo para um container Docker, você pode fazer:

```shell
# substitua MY_CONTAINER pelo nome ou id do seu container Docker:
CONTAINER_HOME=$(docker exec MY_CONTAINER printenv HOME)
docker exec MY_CONTAINER mkdir -p "$CONTAINER_HOME/.codex"
docker cp auth.json MY_CONTAINER:"$CONTAINER_HOME/.codex/auth.json"
```

Se estiver conectado via `ssh` em uma máquina remota, provavelmente vai querer usar [`scp`](https://en.wikipedia.org/wiki/Secure_copy_protocol):

```shell
ssh user@remote 'mkdir -p ~/.codex'
scp ~/.codex/auth.json user@remote:~/.codex/auth.json
```

Ou tente este one-liner:

```shell
ssh user@remote 'mkdir -p ~/.codex && cat > ~/.codex/auth.json' < ~/.codex/auth.json
```

### Conectando via VPS ou remoto

Se você rodar o Codex em uma máquina remota (VPS/servidor) sem navegador local, o helper de login inicia um servidor em `localhost:1455` no host remoto. Para completar o login no seu navegador local, encaminhe essa porta para sua máquina antes de iniciar o fluxo de login:

```bash
# Da sua máquina local
ssh -L 1455:localhost:1455 <user>@<remote-host>
```

Então, nessa sessão SSH, execute `codex` e selecione "Sign in with ChatGPT". Quando solicitado, abra a URL impressa (será `http://localhost:1455/...`) no seu navegador local. O tráfego será tunelado para o servidor remoto.