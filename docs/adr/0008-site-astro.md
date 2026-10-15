# ADR-0008 · Site de apresentação em Astro com mosaico em Canvas

- **Status:** Proposto
- **Data:** 2026-10-14
- **Relacionados:** [Site](../08-site.md) · [Design system](../07-design-system.md)

## Contexto

O site precisa: carregar muito rápido, ter visual escuro e moderno (estilo Vercel/Render/claude.ai), um mosaico interativo que "explode" em azul no clique, botões de download por sistema operacional, link para o GitHub e, futuramente, documentação pública.

## Decisão

- **Astro** gerando site estático, com **Tailwind** e ilhas mínimas de TypeScript.
- Mosaico em **Canvas 2D** (sem bibliotecas), respeitando `prefers-reduced-motion` e pausando fora da tela.
- Links de download resolvidos no build pela API de Releases do GitHub, com detecção de SO no cliente.
- Hospedagem estática (Cloudflare Pages ou GitHub Pages).
- Documentação pública futura com **Starlight** lendo a pasta `docs/`.

## Alternativas consideradas

- **Next.js** — mais pesado do que o necessário para um site de marketing estático.
- **Leptos SSR** (tudo em Rust) — possível, mas o ecossistema de conteúdo/SEO do Astro é muito superior para este caso.
- **WebGL/Three.js para o mosaico** — mais poder visual, mas peso e consumo de bateria maiores; Canvas 2D atinge o efeito desejado.

## Consequências

- ✅ Lighthouse alto e JS mínimo.
- ✅ Contribuidores de front conseguem colaborar sem Rust.
- ⚠️ Mais uma app TypeScript no monorepo (compartilha tokens de design via CSS).
