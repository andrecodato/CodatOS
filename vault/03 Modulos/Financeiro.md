---
tags: [modulo]
status: planejamento
marco: M3
spec: docs/modulos/financeiro.md
---
# Financeiro

Contas, transações, importação OFX/CSV, categorias, metas, visão geral do mês (cartões, fluxo de caixa, rosca por categoria, últimas transações).

**Conceitos:** [[Local-first]] (dados ficam no PC) · [[Cofre]]

## Fios soltos
- Dinheiro em **centavos inteiros**.
- Dedupe na importação OFX (hash de data+valor+descrição+id do banco).
- Modo privacidade (R$ •••••) é essencial no celular.
- Sincronizar ou não? (Q-07) — proposta: só no PC; celular lê ao vivo pelo [[Relay]].

## Pesquisa
- [[Open Finance Brasil]]

## Ideias
- Importador Organizze/Mobills.
- Contas a vencer no calendário da semana.
