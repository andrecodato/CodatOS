# ADR-0006 · Nuvem em NestJS + PostgreSQL + Valkey

- **Status:** Proposto — confirmar na [Q-32](../10-perguntas-em-aberto.md#stack)
- **Data:** 2026-10-14
- **Relacionados:** [Stack → nuvem](../03-stack.md#nuvem-typescript) · [Modelo de dados](../05-modelo-de-dados.md#nuvem-postgresql) · [APIs](../06-api-e-protocolos.md#api-da-nuvem-rest)

## Contexto

A nuvem faz: contas e autenticação (senha derivada, OAuth, 2FA, passkeys), dispositivos, tenants/membros/grupos/permissões, armazenamento de itens cifrados com sincronização, assinaturas e o relay WebSocket. Ela **não** processa conteúdo do usuário (zero-knowledge). O autor sugeriu NestJS com Postgres em Docker.

## Decisão

- **NestJS** (TypeScript, adapter Fastify) organizado em módulos: `auth`, `users`, `devices`, `tenants`, `groups`, `vaults`, `sync`, `relay`, `billing`, `audit`.
- **PostgreSQL** (≥ 17) com **Row-Level Security** e **Drizzle ORM**.
- **Valkey** para presença do relay, pub/sub entre instâncias e *rate limiting*.
- Tudo empacotado em **Docker Compose** (dev e self-host) atrás do **Caddy**.
- Tipos de protocolo gerados a partir do crate Rust `protocol`.

## Alternativas consideradas

- **Rust/Axum** — compartilharia crates com o app e teria excelente desempenho no relay. Mas, como o servidor não faz criptografia de conteúdo, o ganho de compartilhamento é pequeno; o ecossistema Nest (auth, billing, admin, validação) acelera mais. Se o relay virar gargalo, ele pode ser extraído para um serviço Rust isolado sem afetar o resto.
- **Supabase/Firebase** — rápido para começar, mas acopla a um fornecedor e complica self-host e RLS customizada para o modelo de cofres.
- **Prisma** em vez de Drizzle — mais popular, porém RLS com `set_config` por transação é menos natural.

## Consequências

- ✅ Produtividade alta e stack conhecida pelo autor.
- ✅ Self-host simples: `docker compose up`.
- ⚠️ Duas linguagens no projeto; contratos precisam de geração de tipos e testes de contrato no CI.
- ⚠️ WebSocket do relay em Node precisa de testes de carga (meta: 5 mil conexões por instância).
