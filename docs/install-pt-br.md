## Instalação & build

### Requisitos do sistema

| Requisito                 | Detalhes                                                         |
| ------------------------- | ---------------------------------------------------------------- |
| Sistemas operacionais     | macOS 12+, Ubuntu 20.04+/Debian 10+, ou Windows 11 **via WSL2**  |
| Git (opcional, recomendado) | 2.23+ para helpers de PR embutidos                             |
| RAM                       | Mínimo 4 GB (8 GB recomendado)                                  |

### DotSlash

O GitHub Release também contém um arquivo [DotSlash](https://dotslash-cli.com/) para o Codex CLI chamado `codex`. Usar um arquivo DotSlash possibilita fazer um commit leve no controle de versão para garantir que todos os colaboradores usem a mesma versão de um executável, independentemente da plataforma de desenvolvimento.

### Build a partir do código-fonte

```bash
# Clone o repositório e navegue até a raiz do workspace Cargo.
git clone https://github.com/openai/codex.git
cd codex/codex-rs

# Instale a toolchain Rust, se necessário.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup component add rustfmt
rustup component add clippy

# Compile o Codex.
cargo build

# Inicie a TUI com um prompt de exemplo.
cargo run --bin codex -- "explain this codebase to me"

# Após fazer alterações, garanta que o código esteja limpo.
cargo fmt -- --config imports_granularity=Item
cargo clippy --tests

# Execute os testes.
cargo test