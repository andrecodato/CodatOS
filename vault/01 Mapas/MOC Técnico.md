---
tags: [moc]
---
# MOC Técnico

> 📄 Formal: `docs/02-arquitetura.md` · `docs/03-stack.md` · `docs/05-modelo-de-dados.md` · `docs/06-api-e-protocolos.md`
> 🗺️ [[Arquitetura.canvas|Canvas da arquitetura]]

## Peças
- **Shell Tauri 2** — janela, bandeja, autostart, updater
- **UI Leptos (WASM)** — o mesmo bundle no desktop, navegador e PWA
- **Núcleo Rust** → [[Agente local]] — [[Coletor|coletores]], conexões, [[Cofre]], sync, API local
- **SQLite cifrado** (SQLCipher) — fonte da verdade → [[Local-first]]
- **Nuvem NestJS + PostgreSQL + Valkey** — contas, tenants, itens cifrados, billing
- **[[Relay]]** — túnel Noise ponta a ponta para o celular

## Contratos
- API do agente: JSON-RPC sobre WebSocket + canais binários (terminal/SFTP)
- API da nuvem: REST `/v1`
- Sync: pull por cursor, push com `baseRevision`, merge 3-way no cliente
- Crate `protocol` = fonte da verdade, gera tipos TS para a nuvem

## Stack em uma linha
Rust · Tauri 2 · Leptos · Tailwind · Axum · Tokio · sqlx/SQLCipher · sysinfo · bollard · russh · xterm.js · NestJS · Drizzle · PostgreSQL · Valkey · Caddy · Astro

## Pesquisa técnica
- [[Leptos vs alternativas]]
- [[Temperatura no Windows]]
- [[WSL por dentro]]
- [[Docker em múltiplos engines]]
- [[Benchmark Termius]]
- [[Provedores de clima]]
- [[Assinatura de código]]
- [[Open Finance Brasil]]

## Decisões
- [[Log de decisões]]
