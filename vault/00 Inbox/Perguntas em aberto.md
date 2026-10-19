---
tags: [inbox, perguntas]
---
# Perguntas em aberto

> Lista completa (Q-01…Q-35) com propostas padrão: `docs/10-perguntas-em-aberto.md`. Aqui ficam só as que **bloqueiam** algo, para responder primeiro.

Nenhuma pergunta bloqueante em aberto desde 2026-10-15. Pendências práticas:

- [ ] **Q-01** — comprar `codatos.com` (provisório) depois de checar disponibilidade e INPI → [[Site]]
- [ ] **Q-35** — rodar o teste do `cargo-xwin` no início do M0

## Respostas
- [x] **Q-35 — Build Windows:** testar `cargo-xwin` no M0; se falhar, Rust no Windows (2026-10-15)
- [x] **Q-01 — Domínio:** `codatos.com` provisório, ainda não comprado (2026-10-15)
- [x] **Q-13 — Temperatura no Windows:** revisto em 2026-10-17: PawnIO instalado pelo próprio CodatOS e lido pelo serviço `codatos-sensors`, sem programa externo (antes: ler o LibreHardwareMonitor) → [[Temperatura no Windows]]
- [x] **Q-34 — Áreas do TaskHub:** continuam campo dentro do tenant (2026-10-15) → [[TaskHub]]
- [x] **Q-07 — Financeiro:** só no PC, gravado como item de cofre para sync futuro, com export para backup (2026-10-15) → [[Financeiro]]
- [x] **Q-19 — Recuperação:** sem cópia da chave no servidor; chave de recuperação + dispositivo confiável no M4 (2026-10-15) → [[Zero-knowledge]]
- [x] **Q-16 — Pagamento:** Stripe (cartão + Pix); preço perto do M4 (2026-10-15) → [[Planos]]
- [x] **Q-23 — Celular:** três níveis por aparelho (Ver, Controlar padrão, Total com terminal) (2026-10-15) → [[Acesso remoto]]
- [x] **Q-20 — Tenant e sub users:** contextos e empresas; sub user com conta própria (2026-10-15) → [[Tenant]]
- [x] **Q-01 — Nome:** **CodatOS** (2026-10-14)
- [x] **Q-26 — Licença:** **MIT** para todo o projeto (2026-10-14)
- [x] **Q-31 — Front em Rust:** UI escrita em Rust (Leptos) — ADR-0002 aceito (2026-10-14) → [[Leptos vs alternativas]]
- [x] **Q-33 — Pasta `TaskHub/`:** só referência local, fora do repositório (`.gitignore`) (2026-10-14)
- [x] **Q-04 — TaskHub:** refeito na stack Rust, sem ponte com a API antiga (2026-10-14) → [[TaskHub]]
- [x] **Ambiente de testes:** npm, Docker, Rust etc. instalados no **WSL Ubuntu**, não no Windows (2026-10-14)
- [x] **Q-35 — WSL:** Ubuntu 26.04.1 configurado e verificado (2026-10-14) — scripts em `scripts/wsl/`

_(registrar novas respostas aqui com data e depois atualizar `docs/` + [[Log de decisões]])_
