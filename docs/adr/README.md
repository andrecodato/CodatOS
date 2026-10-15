# Decisões de arquitetura (ADRs)

> ↑ [Índice da documentação](../README.md)

Cada ADR registra **uma** decisão: o contexto, o que foi decidido, as alternativas e as consequências. ADRs não são editados depois de aceitos — uma mudança de rumo gera um novo ADR que **substitui** o anterior.

| ADR | Título | Status |
|---|---|---|
| [0001](0001-monorepo.md) | Monorepo com workspace Cargo + pnpm | Proposto |
| [0002](0002-tauri-e-leptos.md) | App desktop com Tauri 2 e UI em Leptos (Rust/WASM) | **Aceito** |
| [0003](0003-agente-local-api-unica.md) | Núcleo local com uma API única para desktop, navegador e celular | Proposto |
| [0004](0004-criptografia-ponta-a-ponta.md) | Criptografia ponta a ponta (zero-knowledge) em vez de hash para segredos | Proposto |
| [0005](0005-local-first.md) | Local-first: dados vivem no dispositivo; a nuvem sincroniza configurações | Proposto |
| [0006](0006-nestjs-postgres.md) | Nuvem em NestJS + PostgreSQL + Valkey | Proposto |
| [0007](0007-relay-noise.md) | Acesso remoto via relay com canal Noise ponta a ponta | Proposto |
| [0008](0008-site-astro.md) | Site de apresentação em Astro com mosaico em Canvas | Proposto |

**Status possíveis:** Proposto → Aceito → (Substituído por ADR-XXXX | Descontinuado)

## Modelo

```markdown
# ADR-NNNN · Título curto no imperativo

- **Status:** Proposto
- **Data:** AAAA-MM-DD
- **Relacionados:** links

## Contexto
O problema e as forças em jogo.

## Decisão
O que vamos fazer.

## Alternativas consideradas
- Opção — por que não.

## Consequências
Positivas, negativas e o que passa a ser necessário.
```
