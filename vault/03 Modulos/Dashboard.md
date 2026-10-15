---
tags: [modulo]
status: planejamento
marco: M1
spec: docs/modulos/dashboard.md
---
# Dashboard

Primeira tela do dia: saudação + busca + notificações + data/hora, e uma grade de [[Widget|widgets]] configurável.

**Layout do mockup v0:** Saldo · Tarefas · Calendário · Tempo / Uso do PC · Discos · Projetos recentes · WSL + Docker.

**Conceitos:** [[Widget]] · [[Coletor]] · [[Local-first]]

## Fios soltos
- Grade própria (Pointer Events) em Leptos — validar no spike do M0 ([[Leptos vs alternativas]]).
- Layout por breakpoint para o celular ficar bom na PWA.
- Múltiplos dashboards por [[Tenant]].

## Ideias
- Alertas por limite em qualquer widget.
- Exportar layout como JSON → galeria da comunidade.
