---
tags: [pesquisa]
status: aberta
data: 2026-10-15
relacionado: Q-35
---
# Spike M0

**Pergunta:** Leptos (CSR) dentro do Tauri 2 funciona, e dá para gerar o app Windows a partir do WSL com `cargo-xwin`?

## O que se sabe

Testado em 2026-10-15 num Ubuntu 24.04 sem tela (Xvfb), Rust 1.99.0, Trunk 0.21.14, `tauri-cli` 2.12.3, Tauri 2.12.4, Leptos 0.8.22, WebKitGTK 2.52.6.

| Item | Resultado |
|---|---|
| UI Leptos com Trunk (`trunk build --release`) | ✅ WASM de 101 KB, **38 KB com gzip** (meta < 2 MB, RNF-DES-05) |
| Janela Tauri com a UI embutida (`cargo tauri build`) | ✅ renderiza e reage a cliques |
| Modo dev (`cargo tauri dev` → `trunk serve` na porta 1420) | ✅ |
| Pacotes Linux | ✅ `.deb` 2,2 MB · AppImage 77 MB |
| Clippy pedantic + `unsafe_code = "deny"` com o Tauri | ✅ sem exceções |
| `cargo-xwin` (app Windows) | ⏳ não testado: a nuvem onde rodei bloqueia `aka.ms` (download do CRT/SDK da Microsoft). Falta rodar no WSL |
| Instalador NSIS gerado no Linux | ⏳ depende do item acima |
| RAM com janela aberta (Linux, Xvfb, render por software) | ~160 MB processo + ~270 MB WebKitWebProcess + ~50 MB rede. Não representa o Windows; a medição oficial é o passo 8 |

### Pegadinhas encontradas

1. **wasm-opt antigo no Trunk 0.21.** O padrão (`version_123`) falha com "bulk memory" porque o Rust ≥ 1.87 liga esses recursos do WASM por padrão. Corrigido fixando `wasm_opt = "version_133"` no `apps/ui/Trunk.toml` e passando os `--enable-*` em `data-wasm-opt-params` no `index.html`.
2. **CSP do Tauri anula `'unsafe-inline'` em `style-src`.** O Tauri injeta hashes dos estilos embutidos na CSP; com hash presente, o navegador ignora `'unsafe-inline'`, e todo `style="…"` que o Leptos aplica some (layout quebrado, sem erro no console). Corrigido com `"dangerousDisableAssetCspModification": ["style-src"]` no `tauri.conf.json`. Scripts continuam com hash/`'self'`. Com Tailwind (passo 5) quase tudo vira classe, mas estilos dinâmicos (posição na grade, largura de barras) seguem precisando de `style`.
3. **`cargo build` puro abre o `devUrl`.** Sem a feature `custom-protocol` (que o `cargo tauri build` liga), o binário tenta `http://127.0.0.1:1420` e mostra "Connection refused". Por isso o `just build-win` usa `cargo tauri build --runner cargo-xwin` em vez de um alias do cargo.
4. O `cwd` de `beforeDevCommand`/`beforeBuildCommand` é relativo à pasta do `tauri.conf.json` (`../../ui` funciona).

## Para fechar a Q-35 (no seu WSL)

```bash
bash scripts/wsl/setup-user.sh               # instala trunk, tauri-cli, cargo-xwin, just
sudo apt-get install -y nsis                 # ou rode de novo o setup-root.sh
just build-win                               # .exe + instalador NSIS
./target/x86_64-pc-windows-msvc/release/codatos.exe   # abre no Windows pela interop
```

O instalador fica em `target/x86_64-pc-windows-msvc/release/bundle/nsis/`. Anotar aqui: se compilou, se a janela abriu com a UI, e o que o WebView2 mostrou no user agent.

## Fontes
- Execução do próprio spike (comandos acima, capturas da janela no Xvfb).
- [Releases do binaryen](https://github.com/WebAssembly/binaryen/releases) (origem do `wasm-opt` version_133).
- Documentação do Tauri sobre CSP e cross-compilação com `cargo-xwin`: não consegui abrir daqui (rede bloqueada); conferir ao fechar a Q-35.

## Opções para o CodatOS
1. `cargo-xwin` funciona no WSL → dev diário dos recursos Windows no WSL; MSI e release no runner Windows da CI.
2. Não funciona → recursos Windows testados só pela CI (runner Windows), com artefato baixado.

**Inclinação:** opção 1, pendente do teste no WSL.

Relacionado: [[App desktop]] · [[Leptos vs alternativas]]
