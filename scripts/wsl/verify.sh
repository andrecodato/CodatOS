#!/usr/bin/env bash
# Confere o ambiente de desenvolvimento do CodatOS no WSL Ubuntu.
# Uso (no PowerShell): wsl -d Ubuntu -e bash -l scripts/wsl/verify.sh
# shellcheck disable=SC1091
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

row() { printf '%-16s %s\n' "$1" "$2"; }
ver() { "$@" 2>/dev/null | head -1 || echo "FALTANDO"; }

echo "---- versoes ----"
row git              "$(ver git --version)"
row gcc              "$(ver gcc --version)"
row clang            "$(ver clang --version)"
row rustc            "$(ver rustc --version)"
row cargo            "$(ver cargo --version)"
row rustup           "$(ver rustup --version)"
row "alvos rust"     "$(rustup target list --installed 2>/dev/null | tr '\n' ' ')"
row trunk          "$(ver trunk --version)"
row tauri-cli        "$(ver cargo tauri --version)"
row cargo-xwin       "$(ver cargo xwin --version)"
row cargo-deny       "$(ver cargo deny --version)"
row just             "$(ver just --version)"
row node             "$(ver node --version)"
row npm              "$(ver npm --version)"
row pnpm             "$(ver pnpm --version)"
row docker           "$(ver docker --version)"
row "docker compose" "$(ver docker compose version)"
row webkit2gtk       "$(pkg-config --modversion webkit2gtk-4.1 2>/dev/null || echo FALTANDO)"

echo "---- git ----"
row defaultBranch "$(git config --global init.defaultBranch)"
row autocrlf      "$(git config --global core.autocrlf)"
row user.name     "$(git config --global user.name || echo '(nao definido)')"
row user.email    "$(git config --global user.email || echo '(nao definido)')"

echo "---- docker ----"
row servico "$(systemctl is-active docker 2>/dev/null)"
if id -nG | tr ' ' '\n' | grep -qx docker; then
  row "grupo docker" sim
  row hello-world "$(docker run --rm hello-world 2>&1 | grep -m1 -o 'Hello from Docker!' || echo FALHOU)"
else
  row "grupo docker" "nao — rode setup-root.sh e reabra o terminal do Ubuntu"
fi

echo "---- compilacao rapida (Rust nativo e wasm32) ----"
tmp="$(mktemp -d)"
(
  cd "$tmp" && cargo new --quiet hello >/dev/null 2>&1 && cd hello || exit 1
  cargo build --quiet && row "cargo build" "$(./target/debug/hello)"
  cargo build --quiet --target wasm32-unknown-unknown \
    && row "wasm32 build" "$([ -f target/wasm32-unknown-unknown/debug/hello.wasm ] && echo ok || echo FALHOU)"
)
rm -rf "$tmp"
