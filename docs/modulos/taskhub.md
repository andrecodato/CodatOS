# Módulo · TaskHub

> **Prefixo:** `RF-TASK` · **Marco principal:** M3 · **Status:** Rascunho — revisado em 2026-10-14 (TaskHub analisado; decisão: refazer em Rust)
> ↑ [Requisitos](../01-requisitos.md) · Relacionados: [Dashboard](dashboard.md) · [Projetos](projetos.md) · [Integrações → Calendário](integracoes.md#calendário) · [Multitenancy](multitenancy.md)

## Objetivo

**Refazer o TaskHub na stack do CodatOS** (núcleo Rust + UI Leptos), usando o projeto atual — pasta local `TaskHub/`, fora do repositório — apenas como **referência funcional**. O módulo deve ter a mesma linguagem visual do app, funcionar offline e se integrar ao dashboard, ao calendário e aos projetos, **sem perder** o que o TaskHub já faz bem (campos personalizáveis e sincronização com o GitHub Projects).

## O TaskHub hoje

| Aspecto | Situação atual |
|---|---|
| Nome nos pacotes | `kanban-academico` (front) e `taskhub-api` (API) |
| Front | Next.js 16, React 19, Tailwind CSS 4, dnd-kit, lucide-react |
| API | NestJS 11 + Prisma 6 + PostgreSQL 16 (Docker), autenticação JWT com senhas em bcrypt |
| Modos de armazenamento | **Database** (Postgres), **Híbrido** (Postgres + sync com GitHub) e **Só GitHub Projects** |
| Integração | GitHub Projects v2 via GraphQL com PAT; tabela de mapeamento de IDs externos |
| Modelo | Workspace (membros OWNER/MEMBER) → Projetos → Tarefas; **campos personalizáveis por workspace**: Status, Área, Matéria, Categoria — com templates (ex.: "acadêmico") |
| Visões | Kanban com arrastar e soltar, Tabela, Roadmap (início/vencimento), detalhe da tarefa |
| Fluxo padrão | Backlog → Todo → In Progress → Test e QA → Done |
| Áreas padrão | Estágio, Faculdade, Pessoal |
| Portas | Front 4317, API 3010, Postgres 5434 |
| Ideia registrada | Importar trabalhos e prazos do Google Classroom |

### Pontos de atenção na incorporação

- O **PAT do GitHub** e o **token de sessão** ficam no `localStorage` do navegador. No CodatOS, o PAT vai para o [cofre](../04-seguranca.md#7-segurança-local) e as chamadas ao GitHub são feitas pelo núcleo (R-02).
- **Senhas com bcrypt e JWT próprios** deixam de existir: no modo local não há login; com conta, vale a autenticação do CodatOS (Argon2id + KDF no cliente).
- Os dados vivem num **Postgres em Docker**. No CodatOS ficam no **SQLite local** ([ADR-0005](../adr/0005-local-first.md)) — usar tarefas não pode exigir Docker rodando.
- A pasta `TaskHub/` tem `.git` próprio e fica **fora do repositório** do CodatOS (`/TaskHub/` no `.gitignore`) — [Q-33](../10-perguntas-em-aberto.md#taskhub).

## Mapeamento TaskHub → CodatOS

| TaskHub | CodatOS |
|---|---|
| Workspace + membros (OWNER/MEMBER) | [Tenant](multitenancy.md) + membros e grupos |
| Área (Estágio/Faculdade/Pessoal) | Campo da tarefa dentro do tenant ([Q-34](../10-perguntas-em-aberto.md#taskhub)) |
| FieldDefinition / FieldOption | Campos personalizados por tenant |
| Project | Projeto de tarefas (vinculável a um repositório local — [RF-PROJ-006](projetos.md#requisitos-funcionais)) |
| Task | Tarefa |
| SyncConnection + ExternalMapping | Provedor GitHub Projects + tabela de mapeamentos |
| StorageMode | Modo de sincronização **por projeto**: Local · Híbrido · Só GitHub |
| JWT + bcrypt | Nenhum (modo local) ou conta do CodatOS |

## Estratégia: reescrita em Rust

Decidido em 2026-10-14 ([Q-04](../10-perguntas-em-aberto.md#taskhub)): **nada do código atual é reaproveitado diretamente** — nem ponte com a API antiga.

| Camada do TaskHub | Vira no CodatOS |
|---|---|
| Domínio e API NestJS + Prisma | Serviço de tarefas no núcleo Rust, com SQLite local |
| UI Next.js/React (`kanban-board`, `table-view`, `roadmap-view`, `task-detail-modal`) | Telas em Leptos — os componentes atuais servem de **referência de UX** |
| Adaptador GitHub Projects (TypeScript/GraphQL) | Provedor `GitHubProjects` em Rust; PAT no cofre |
| Templates de campos ("acadêmico") | Templates de campos do CodatOS |
| JWT + bcrypt | Nenhum (modo local) ou conta do CodatOS |

Todos os provedores implementam a interface `TaskProvider` do núcleo (Local e GitHub Projects na 1.0; depois Todoist, Microsoft To Do, GitHub Issues). Um importador opcional dos dados atuais (RF-TASK-007) evita recriar tarefas à mão.

## Telas (mockup v0)

Abas **Minhas tarefas · Projetos · Quadros** e botão **+ Nova tarefa**.

- Lista com busca e filtros (Todas, Prioridade, Projeto); cada tarefa tem checkbox, título, prioridade (Alta/Média/Baixa), vencimento (Hoje, Amanhã, data) e projeto.
- Painel lateral **Resumo**: total, em andamento, concluídas; **Projetos** com contagem de tarefas.
- A visão **Roadmap** e a edição de **Campos** do TaskHub atual ainda não estão no mockup — desenhar.

## Requisitos funcionais

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-TASK-001 | Listar "Minhas tarefas" com busca, filtros (status, prioridade, projeto, área, matéria, categoria, vencimento) e ordenação. | Deve | M3 |
| RF-TASK-002 | Criar, editar, concluir, reabrir e excluir tarefas: título, descrição (Markdown), status, prioridade, data de início, vencimento, projeto, campos personalizados, subtarefas. | Deve | M3 |
| RF-TASK-003 | Projetos com cor/ícone e contagem de tarefas abertas. | Deve | M3 |
| RF-TASK-004 | **Kanban**: colunas = opções do campo Status; arrastar e soltar entre colunas e reordenar dentro da coluna. | Deve | M3 |
| RF-TASK-005 | Widget **Tarefas** no dashboard: total, em andamento, concluídas, barra de progresso e próximas tarefas com vencimento relativo. | Deve | M3 |
| RF-TASK-006 | Lembretes e notificações de vencimento. | Deveria | M3 |
| RF-TASK-007 | **Importador do TaskHub** (opcional): workspaces, campos e opções (cores, ordem, arquivadas), projetos, tarefas, datas e mapeamentos do GitHub — direto do Postgres ou de um export JSON; idempotente. | Poderia | M3 |
| RF-TASK-008 | Captura rápida pela paleta de comandos ("nova tarefa: …"). | Deveria | M3 |
| RF-TASK-009 | Vincular tarefas a projetos locais ([RF-PROJ-006](projetos.md#requisitos-funcionais)) e exibir vencimentos no calendário da semana. | Poderia | M3 |
| RF-TASK-010 | Atribuição de tarefas a membros do tenant e comentários (colaborativo). | Poderia | M6 |
| RF-TASK-011 | Provedores externos adicionais (Todoist, Microsoft To Do, GitHub Issues). | Poderia | Futuro |
| RF-TASK-012 | **Campos personalizados** por tenant (Status, Área, Matéria, Categoria e novos), com opções coloridas, ordenáveis e arquiváveis; **templates** de campos (ex.: "Acadêmico", "Trabalho"). | Deve | M3 |
| RF-TASK-013 | Visão **Tabela** com colunas configuráveis e edição inline. | Deve | M3 |
| RF-TASK-014 | Visão **Roadmap** (linha do tempo por início/vencimento). | Deveria | M3 |
| RF-TASK-015 | **Sync com GitHub Projects v2** por projeto, nos modos Local, Híbrido (bidirecional) e Só GitHub; criação de projeto no GitHub pelo app; PAT guardado no cofre; chamadas feitas pelo núcleo. | Deve | M3 |
| ~~RF-TASK-016~~ | ~~Ponte com a API atual do TaskHub~~ — descartado: o módulo será refeito em Rust ([Q-04](../10-perguntas-em-aberto.md#taskhub)). | — | — |
| RF-TASK-017 | Importar trabalhos e prazos do Google Classroom como tarefas (área "Faculdade"). | Poderia | Futuro |

## Regras de negócio

- Tarefas são **dados**: ficam no SQLite local. Entre dispositivos do CodatOS, só sincronizam **cifradas ponta a ponta** ([Q-05](../10-perguntas-em-aberto.md#taskhub)).
- Sincronizar com o **GitHub Projects** é uma escolha explícita por projeto: os dados daquele projeto passam a existir também no GitHub — a UI avisa isso ao ativar.
- No modo Híbrido, conflitos são resolvidos por campo comparando a data de alteração local com a do GitHub; mapeamentos de IDs ficam em tabela própria.
- As colunas do Kanban derivam do campo Status do tenant; arquivar uma opção não apaga tarefas.
- Tarefas colaborativas (RF-TASK-010) vivem num cofre compartilhado do tenant.

## Critérios de aceite

- **Dado** o banco atual do TaskHub, **quando** o importador roda, **então** projetos, tarefas, campos (com cores e ordem), datas e mapeamentos do GitHub são preservados; rodar de novo não duplica nada.
- **Dado** um projeto em modo Híbrido, **quando** um card é movido no CodatOS, **então** o status muda no GitHub Projects em até 10 s; e uma mudança feita no GitHub aparece no próximo ciclo de sync.
- **Dado** um PAT configurado, **então** ele está cifrado no cofre e nunca aparece em `localStorage`, logs ou na UI.
- **Dado** tarefas com vencimento hoje e amanhã, **quando** o widget é exibido, **então** aparecem como "Hoje" e "Amanhã", ordenadas por vencimento e prioridade.

## Perguntas em aberto

- ✅ Refazer em Rust, sem ponte → [Q-04](../10-perguntas-em-aberto.md#taskhub)
- ✅ Pasta `TaskHub/` fora do repositório → [Q-33](../10-perguntas-em-aberto.md#taskhub)
- Tarefas sincronizam entre dispositivos (cifradas)? → [Q-05](../10-perguntas-em-aberto.md#taskhub)
- ✅ "Área" continua campo → [Q-34](../10-perguntas-em-aberto.md#taskhub)
