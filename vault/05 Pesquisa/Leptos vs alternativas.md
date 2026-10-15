---
tags: [pesquisa]
status: aberta
relacionado: Q-31
---
# Leptos vs alternativas (UI)

Pedido: "front em Rust, nativo em Windows/Linux/macOS, mas moderno e bonito como o mockup" **+** acessível no navegador.

## Por que não GUI nativa (egui/iced/Slint/GPUI)
Atingem "nativo em Rust", mas não servem a mesma UI no navegador/celular e dificultam o visual do mockup.

## Candidatos com WebView (Tauri)
|                                   | Leptos (Rust/WASM)          | React/Svelte (TS)                   |
| --------------------------------- | --------------------------- | ----------------------------------- |
| Atende "front em Rust"            | ✅                           | ⚠️ só o shell/núcleo                |
| Componentes prontos               | Poucos                      | Muitos (shadcn/ui, gráficos, grids) |
| Terminal                          | xterm.js via `wasm-bindgen` | xterm.js nativo                     |
| Tipos compartilhados com o núcleo | ✅ direto                    | via geração de TS                   |
| Velocidade de desenvolvimento     | Média                       | Alta                                |

## Spike do M0 (critérios para manter o Leptos)
- [ ] Anel de CPU em SVG atualizando a cada 2 s, sem *jank*
- [ ] xterm.js recebendo frames binários do WebSocket com `vim`/`htop` fluidos
- [ ] Grade de 12 widgets arrastáveis/redimensionáveis a 60 fps
- [ ] Bundle WASM < 2 MB gzip; rebuild incremental < 5 s

## ✅ Decisão (2026-10-14)
**UI escrita em Rust (Leptos)** — ADR-0002 aceito. React/Svelte descartado por decisão do autor; o spike acima continua útil para **descobrir cedo** onde vamos precisar de componentes próprios (gráficos, grade, tabela virtualizada), não mais para escolher a stack.

📄 `docs/adr/0002-tauri-e-leptos.md` · [[Agente local]]
