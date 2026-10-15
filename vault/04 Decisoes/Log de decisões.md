---
tags: [decisao]
---
# Log de decisões

> Decisões formais viram ADR em `docs/adr/`. Aqui fica o registro rápido — inclusive das pequenas — com data e status.

| Data       | Decisão                                                                                  | Status                     | Onde                                                                     |
| ---------- | ---------------------------------------------------------------------------------------- | -------------------------- | ------------------------------------------------------------------------ |
| 2026-10-14 | Monorepo com workspace Cargo + pnpm                                                      | Proposto                   | `docs/adr/0001-monorepo.md`                                              |
| 2026-10-14 | Tauri 2 + UI em Leptos                                                                   | ✅ Aceito — Q-31            | `docs/adr/0002-tauri-e-leptos.md` · [[Leptos vs alternativas]]           |
| 2026-10-14 | Uma API local para janela, navegador e celular                                           | Proposto                   | `docs/adr/0003-agente-local-api-unica.md` · [[Agente local]]             |
| 2026-10-14 | Hash só para login; criptografia ponta a ponta para segredos e configs                   | Proposto                   | `docs/adr/0004-criptografia-ponta-a-ponta.md` · [[Hash vs criptografia]] |
| 2026-10-14 | Local-first: dados no PC, nuvem só configs                                               | Proposto                   | `docs/adr/0005-local-first.md` · [[Local-first]]                         |
| 2026-10-14 | NestJS + PostgreSQL + Valkey + Drizzle                                                   | Proposto — Q-32            | `docs/adr/0006-nestjs-postgres.md`                                       |
| 2026-10-14 | Relay com Noise ponta a ponta para o celular                                             | Proposto                   | `docs/adr/0007-relay-noise.md` · [[Relay]]                               |
| 2026-10-14 | Site em Astro, mosaico em Canvas 2D                                                      | Proposto                   | `docs/adr/0008-site-astro.md` · [[Site]]                                 |
| 2026-10-14 | PWA (não app nativo) para o celular na 1.0                                               | Proposto — Q-24            | `docs/modulos/acesso-remoto.md`                                          |
| 2026-10-14 | Financeiro fica só no PC por padrão                                                      | Proposto — Q-07            | [[Financeiro]]                                                           |
| 2026-10-14 | Domínio `codatos.dev` e porta local 47800 (provisórios)                                  | Provisório — Q-01          | `docs/README.md`                                                         |
| 2026-10-14 | ~~TaskHub: ponte pela API atual no M1 → migração no M3~~                                 | Substituído (linha abaixo) | —                                                                        |
| 2026-10-14 | **TaskHub refeito na stack Rust**; o projeto atual é só referência funcional (sem ponte) | ✅ Decidido — Q-04          | `docs/modulos/taskhub.md` · [[TaskHub]]                                  |
| 2026-10-14 | Pasta `TaskHub/` fora do repositório (`/TaskHub/` no `.gitignore`)                       | ✅ Decidido — Q-33          | `.gitignore`                                                             |
| 2026-10-14 | Nome do produto: **CodatOS**                                                             | ✅ Decidido — Q-01          | `docs/README.md`                                                         |
| 2026-10-14 | Licença **MIT** para todo o projeto                                                      | ✅ Decidido — Q-26          | `LICENSE`                                                                |
| 2026-10-14 | UI escrita em Rust (Leptos) — ADR-0002 aceito                                            | ✅ Decidido — Q-31          | `docs/adr/0002-tauri-e-leptos.md` · [[Leptos vs alternativas]]           |
| 2026-10-14 | Ferramentas de build/teste (Rust, Node/pnpm, Docker) no **WSL Ubuntu**                   | ✅ Decidido                 | `docs/03-stack.md#ambiente-de-desenvolvimento`                           |
| 2026-10-14 | PAT do GitHub sai do `localStorage` e vai para o cofre                                   | Proposto                   | [[TaskHub]] · [[Cofre]]                                                  |
