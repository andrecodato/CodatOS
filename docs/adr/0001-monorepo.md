# ADR-0001 · Monorepo com workspace Cargo + pnpm

- **Status:** Aceito (2026-10-17, no fechamento do M0)
- **Data:** 2026-10-14
- **Relacionados:** [Arquitetura → estrutura](../02-arquitetura.md#estrutura-do-monorepo-planejada) · [Stack](../03-stack.md)

## Contexto

O projeto tem quatro entregáveis (app desktop, UI, nuvem, site) em duas linguagens (Rust e TypeScript) que compartilham contratos: API do agente, formato de sincronização e protocolo do relay. É um projeto open source mantido inicialmente por uma pessoa.

## Decisão

Um único repositório:

- **Workspace Cargo** na raiz com `apps/desktop`, `apps/ui` e `crates/*`.
- **Workspace pnpm** com `apps/cloud` e `apps/site`.
- O crate `protocol` é a fonte da verdade dos contratos e gera tipos TypeScript consumidos pela nuvem.
- CI com filtros por caminho: mudanças só no site não disparam o build do app.

## Alternativas consideradas

- **Repositórios separados** — contratos divergem facilmente; PRs que mudam protocolo + app + nuvem ficam fragmentados; mais fricção para contribuidores.
- **Nx/Turborepo para tudo** — acrescenta complexidade; o Cargo já resolve o lado Rust.

## Consequências

- ✅ Uma mudança de protocolo é um único PR, testado de ponta a ponta.
- ✅ Contribuidores clonam um repositório só.
- ⚠️ CI precisa de cache e filtros bem configurados para não ficar lento.
- ⚠️ Releases independentes por componente (tags `app-v*`, `cloud-v*`, `site-v*`) via release-please.
