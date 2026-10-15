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
- ✅ Q-07: só no PC (celular lê ao vivo pelo [[Relay]]), gravado como item de cofre para sync futuro; export cifrado e CSV/OFX para backup.

## Pesquisa
- [[Open Finance Brasil]]

## Ideias
- Importador Organizze/Mobills.
- Contas a vencer no calendário da semana.
