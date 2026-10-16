# Atalhos de desenvolvimento do CodatOS. Rode `just` para listar.

default:
    @just --list

# App em modo dev: Trunk serve + janela Tauri com recarga
dev:
    cd apps/desktop && cargo tauri dev

# Formata todo o código Rust
fmt:
    cargo fmt --all

# O que a CI roda: formatação, clippy sem warnings e testes
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy -p codatos-ui --target wasm32-unknown-unknown -- -D warnings
    cargo test --workspace

# Licenças, advisories e origem das dependências
deny:
    cargo deny check

# App Windows (.exe + instalador NSIS) compilado no WSL com cargo-xwin (Q-35).
# Usa o cargo tauri: um `cargo build` puro abriria a janela no devUrl, não na UI embutida.
build-win:
    cd apps/desktop && cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc

# App Linux (.deb + AppImage)
build-linux:
    cd apps/desktop && cargo tauri build --bundles deb,appimage
