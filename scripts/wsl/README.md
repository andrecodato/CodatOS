# Ambiente de desenvolvimento no WSL Ubuntu

Scripts que preparam um Ubuntu (WSL2) para desenvolver o CodatOS. As ferramentas ficam **dentro do WSL**, nunca no Windows hospedeiro (ver [Stack → Ambiente de desenvolvimento](../../docs/03-stack.md#ambiente-de-desenvolvimento)).

| Script | Executa como | O que faz |
|---|---|---|
| `setup-root.sh <usuario>` | root | Pacotes base e git, dependências do Tauri para Linux, clang/lld (cross-compilação Windows), Node.js 24 (NodeSource), pnpm, Docker Engine + Compose (repositório oficial; usa os pacotes do Ubuntu se a Docker ainda não publicar a versão) e coloca o usuário no grupo `docker` |
| `setup-user.sh` | seu usuário | rustup + Rust stable, alvos `wasm32-unknown-unknown` e `x86_64-pc-windows-msvc`, ferramentas do M0 com versões fixadas (Trunk, `tauri-cli`, `cargo-xwin`, `cargo-deny`, `just`), preferências do git (`init.defaultBranch=main`, `core.autocrlf=input`) |
| `verify.sh` | seu usuário | Mostra versões, testa o Docker sem sudo e compila um "hello world" nativo e em wasm32 |

## Como rodar (PowerShell, na raiz do repositório)

```bash
wsl -d Ubuntu -u root -e bash scripts/wsl/setup-root.sh "$(wsl -d Ubuntu -e whoami)"
```

```bash
wsl -d Ubuntu -e bash scripts/wsl/setup-user.sh
```

```bash
wsl -d Ubuntu -e bash -l scripts/wsl/verify.sh
```

Os scripts são idempotentes: rodar de novo só atualiza o que faltar. A identidade do git (`user.name`/`user.email`) não é configurada por eles.

## Resultado em 2026-10-14 (Ubuntu 26.04.1 LTS)

git 2.53 · gcc 15.2 · clang 21.1 · Rust 1.99.0 (rustup 1.29) · Node 24.22 · npm 11.19 · pnpm 12.11 · Docker 29.9 · Compose 5.6 · WebKitGTK 2.52.
