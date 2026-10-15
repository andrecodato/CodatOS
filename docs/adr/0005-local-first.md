# ADR-0005 · Local-first: dados vivem no dispositivo; a nuvem sincroniza configurações

- **Status:** Proposto
- **Data:** 2026-10-14
- **Relacionados:** [Requisitos R-01 e R-07](../01-requisitos.md#4-restrições-invioláveis) · [Modelo de dados](../05-modelo-de-dados.md) · [Financeiro](../modulos/financeiro.md)

## Contexto

O servidor não deve armazenar arquivos e deve guardar apenas configurações. Ao mesmo tempo, o usuário quer ver tudo no celular — inclusive dados do PC (métricas, financeiro, tarefas).

## Decisão

- O **banco local** (SQLite cifrado) é a fonte da verdade. O app é completo sem internet e sem conta.
- A nuvem sincroniza **configurações** (conexões, chaves, layouts, preferências, integrações), sempre cifradas ponta a ponta.
- **Dados** (financeiro, histórico de métricas, logs, arquivos) **não** sincronizam por padrão. O celular os vê **ao vivo pelo PC**, via relay — sem armazenamento na nuvem.
- Tarefas e financeiro só sincronizarão se aprovado explicitamente ([Q-05](../10-perguntas-em-aberto.md#taskhub), [Q-07](../10-perguntas-em-aberto.md#financeiro)) e sempre como itens cifrados.

## Alternativas consideradas

- **Nuvem como fonte da verdade (SaaS clássico)** — exigiria guardar dados de usuários no servidor, contrariando o briefing, e tornaria o app inútil offline.
- **Sem nuvem alguma** — perderia sync e acesso remoto, que são o modelo de sustentabilidade.

## Consequências

- ✅ Privacidade por arquitetura; servidor barato (só blobs pequenos).
- ✅ App rápido: toda leitura é local.
- ⚠️ Ver dados pelo celular exige o PC ligado e online.
- ⚠️ Perder o PC sem backup local perde os dados não sincronizados — por isso o export cifrado local ([RF-APP-013](../modulos/app-e-configuracoes.md#requisitos-funcionais)).
