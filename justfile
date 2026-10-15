# Atalhos de desenvolvimento do CodatOS. Rode `just` para listar.

default:
    @just --list

# Formata todo o código Rust
fmt:
    cargo fmt --all

# O que a CI roda: formatação, clippy sem warnings e testes
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

# Licenças, advisories e origem das dependências
deny:
    cargo deny check

# App Windows compilado no WSL com cargo-xwin (Q-35)
build-win:
    cargo xwin-build -p codatos-desktop --release
