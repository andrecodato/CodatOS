---
tags: [conceito, produto]
aliases: [Widgets]
---
# Widget

Cartão do [[Dashboard]] que mostra dados de um módulo e leva a ele com "Ver todos".

- Grade de 12 colunas; arrastar, redimensionar, configurar por instância.
- Só existe se a plataforma suporta a fonte (WSL só no Windows).
- Atualiza por streaming **apenas quando visível** → [[Coletor]].
- Estados padrão: carregando · vazio · erro · sem permissão · desatualizado.
- O **layout** é configuração (sincroniza); os **dados** não → [[Local-first]].

**Futuro:** SDK de widgets da comunidade (plugins WASM em sandbox).

📄 `docs/modulos/dashboard.md#catálogo-de-widgets`
