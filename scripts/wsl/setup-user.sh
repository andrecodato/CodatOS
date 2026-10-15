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

echo "==> preferencias globais do git (sem identidade)"
git config --global init.defaultBranch main
git config --global core.autocrlf input

echo "==> concluido"
