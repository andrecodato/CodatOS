#!/usr/bin/env bash
# Toolchain Rust e preferencias do git para o usuario de desenvolvimento.
set -euo pipefail

if [ ! -x "$HOME/.cargo/bin/rustup" ]; then
  echo "==> rustup (instalador oficial de https://sh.rustup.rs)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile default --default-toolchain stable
fi
# shellcheck disable=SC1091
. "$HOME/.cargo/env"

echo "==> toolchain stable + alvos usados pelo CodatOS"
rustup toolchain install stable --profile default
rustup default stable
# wasm32: UI em Leptos · x86_64-pc-windows-msvc: app Windows via cargo-xwin
rustup target add wasm32-unknown-unknown x86_64-pc-windows-msvc

# Ferramentas do M0 com versoes fixadas. Para atualizar, troque aqui e rode de novo.
# (o toolchain Rust em si fica fixado em rust-toolchain.toml, na raiz do repo)
CARGO_TOOLS=(
  "trunk 0.21.14"      # build da UI Leptos
  "tauri-cli 2.12.3"   # cargo tauri dev/build
  "cargo-xwin 0.23.1"  # app Windows a partir do WSL (Q-35)
  "cargo-deny 0.20.2"  # licencas e advisories
  "just 1.58.0"        # atalhos do justfile
)
echo "==> ferramentas cargo fixadas (compila na primeira vez; pode demorar)"
for entry in "${CARGO_TOOLS[@]}"; do
  read -r crate version <<<"$entry"
  if cargo install --list | grep -qx "$crate v$version:"; then
    echo "    $crate $version ja instalado"
  else
    cargo install --locked "$crate@$version"
  fi
done

echo "==> preferencias globais do git (sem identidade)"
git config --global init.defaultBranch main
git config --global core.autocrlf input

echo "==> concluido"
