# ADR-0002 · App desktop com Tauri 2 e UI em Leptos (Rust/WASM)

- **Status:** Aceito em 2026-10-14 — UI escrita em Rust ([Q-31](../10-perguntas-em-aberto.md#stack))
- **Data:** 2026-10-14
- **Relacionados:** [Stack](../03-stack.md) · [ADR-0003](0003-agente-local-api-unica.md) · [Design system](../07-design-system.md)

## Contexto

Requisitos que puxam em direções diferentes:

1. "Front em Rust", nativo em Windows, Linux e macOS.
2. Visual moderno no nível do mockup v0 (gradientes, gráficos, animações suaves).
3. **A mesma interface acessível pelo navegador** e pelo celular.
4. Leve o suficiente para iniciar com o sistema e ficar residente.

GUIs nativas em Rust (egui, iced, Slint, GPUI) atendem 1 e 4, mas dificultam 2 e não atendem 3 (não rodam como página web servida pelo PC). Electron atende 2 e 3, mas falha em 1 e 4.

## Decisão

- **Tauri 2** como shell nativo (usa a WebView do SO: WebView2, WKWebView, WebKitGTK).
- **Leptos** (CSR, compilado para WASM) como framework de UI — a interface é escrita em Rust e o **mesmo bundle** roda na janela do Tauri, no navegador e na PWA.
- **Tailwind CSS** para estilo; **xterm.js** via `wasm-bindgen` para o terminal (não há emulador maduro em Rust para a web).

## Alternativas consideradas

- **Tauri + React/Svelte (TypeScript)** — ecossistema de UI muito maior (shadcn/ui, bibliotecas de gráficos, grid de dashboard prontos) e desenvolvimento mais rápido. Descartado por decisão do autor: a interface deve ser escrita em Rust. Como a API do núcleo é independente, uma troca futura afetaria só `apps/ui` — mas exigiria um novo ADR.
- **Dioxus** — também compila para desktop e web; preterido pela maturidade dos plugins do Tauri (autostart, updater, bandeja, single-instance, assinatura).
- **egui/iced/Slint** — ótimos para ferramentas, mas sem caminho para servir a UI no navegador.
- **Electron** — consumo de memória incompatível com app residente.

## Consequências

- ✅ Rust de ponta a ponta no app; tipos compartilhados entre UI e núcleo via crate `protocol`.
- ✅ Um único código de UI para desktop, navegador e celular.
- ⚠️ Menos componentes prontos: gráficos, grade de dashboard e tabelas virtualizadas serão componentes próprios.
- ⚠️ Diferenças entre WebViews (principalmente WebKitGTK no Linux) exigem testes visuais nos 3 SOs.
- ⚠️ Interop com JS (xterm.js) precisa de uma camada `wasm-bindgen` bem isolada.
