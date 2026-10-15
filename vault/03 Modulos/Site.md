---
tags: [modulo]
status: planejamento
marco: S1
spec: docs/08-site.md
---
# Site

Landing escura estilo Vercel/Render/claude.ai: hero com **mosaico interativo** (onda de choque azul no clique), screenshot, bento de recursos, privacidade, open source, preços, download por SO, GitHub.

## Protótipo ✅
`apps/site/prototypes/mosaico.html` — HTML puro, abre com duplo clique. Tem painel **Ajustes** para calibrar o efeito ao vivo e copiar os parâmetros. Medido: 0,1–0,4 ms/quadro; 1,1 ms com 14 ondas.

## Fios soltos
- Astro + Tailwind, Canvas 2D, < 40 KB de JS na landing.
- Mosaico: ladrilhos 36 px, onda 0,9 px/ms, pulso 650 ms, rampa `#1D4ED8 → #3B82F6 → #93C5FD`.
- `prefers-reduced-motion` → estático.
- Downloads via GitHub Releases + detecção de SO.
- S1 "em breve" junto com o M1; S2 completo na 1.0.

## Ideias
- Demo interativa do dashboard (dados falsos) direto no site — dá para reaproveitar a UI WASM!
- Página `/security` explicando zero-knowledge com animação do cadeado.
