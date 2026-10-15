---
tags: [modulo]
status: planejamento
marco: M3
spec: docs/modulos/taskhub.md
---
# TaskHub

Kanban de tarefas **que já existe** (pasta `TaskHub/`, pacote `kanban-academico`) e será incorporado ao CodatOS.

## Como ele é hoje (analisado em [[2026-10-14]])
- **Front:** Next.js 16 + React 19 + Tailwind 4 + dnd-kit + lucide
- **API:** NestJS 11 + Prisma 6 + Postgres 16 (Docker) · JWT + bcrypt
- **Modos:** Database · Híbrido · Só GitHub Projects (GraphQL v2 via PAT)
- **Modelo:** Workspace (OWNER/MEMBER) → Projetos → Tarefas; campos personalizáveis Status/Área/Matéria/Categoria com templates ("acadêmico")
- **Visões:** Kanban, Tabela, Roadmap, detalhe
- **Fluxo:** Backlog → Todo → In Progress → Test e QA → Done
- **Áreas:** Estágio · Faculdade · Pessoal
- **Ideia guardada:** importar prazos do Google Classroom

## ✅ Estratégia: reescrita em Rust (decidido em 2026-10-14)
O TaskHub atual é **só referência funcional** — a pasta `TaskHub/` fica fora do repositório.
- Domínio em Rust/SQLite no núcleo; UI em Leptos usando os componentes React atuais **como referência de UX**.
- Adaptador GitHub Projects reescrito em Rust; PAT no [[Cofre]].
- Sem ponte com a API antiga; importador dos dados atuais é opcional.

Interface `TaskProvider`: Local · GitHub Projects · (Todoist, MS To Do, GitHub Issues).

## Ajustes de segurança na incorporação
- PAT e token de sessão saem do `localStorage` → PAT no [[Cofre]], chamadas pelo [[Agente local]].
- bcrypt/JWT próprios somem → conta do CodatOS ([[Zero-knowledge]]) ou nenhuma no modo local.
- Tarefas sem precisar de Docker/Postgres → [[Local-first]].

## Mapeamento
- Workspace → [[Tenant]] · Áreas → campo (Q-34 ✅)
- SyncConnection/ExternalMapping → provedor GitHub + mapeamentos

## Em aberto
- [x] Q-04 → refazer em Rust
- [x] Q-33 → pasta fora do repositório
- [x] Q-31 → UI em Leptos ([[Leptos vs alternativas]])
- [ ] Q-05 sync E2E entre dispositivos
- [x] Q-34 → Área é campo

[[Perguntas em aberto]] · [[Dashboard]] (widget Tarefas)
