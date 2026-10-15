# Módulo · Projetos

> **Prefixo:** `RF-PROJ` · **Marco principal:** M1 · **Status:** Rascunho
> ↑ [Requisitos](../01-requisitos.md) · Relacionados: [Dashboard](dashboard.md) · [TaskHub](taskhub.md) · [Conexões](conexoes.md) · [Sistema → Docker](sistema.md#docker)

## Objetivo

Responder "em que eu estava trabalhando?" em um segundo: descobrir repositórios no PC (e dentro das distros WSL), ordenar por uso recente, mostrar o estado do git e abrir tudo com um clique.

## Telas

- Widget **Projetos recentes** (mockup v0): nome, ícone, "último acesso: hoje / ontem / 2 dias atrás".
- Página **Projetos**: lista/grade com busca, filtros (pasta raiz, linguagem, com alterações pendentes), favoritos e detalhes.

## Requisitos funcionais

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-PROJ-001 | Pastas raiz configuráveis (Windows/macOS/Linux e caminhos dentro de distros WSL) para descobrir repositórios git, com profundidade máxima e padrões ignorados (`node_modules`, `target`, `.venv`…). | Deve | M1 |
| RF-PROJ-002 | Importar projetos recentes de editores: VS Code, Cursor, JetBrains (e Visual Studio — Poderia). | Deveria | M1 |
| RF-PROJ-003 | Lista ordenada por último acesso/commit: nome, caminho, branch, alterações pendentes, ahead/behind, último commit, linguagem principal. | Deve | M1 |
| RF-PROJ-004 | Ações: abrir no editor (configurável), abrir terminal, abrir no explorador, copiar caminho, abrir o remoto no navegador (GitHub/GitLab). | Deve | M1 |
| RF-PROJ-005 | Favoritar/fixar, tags e ocultar projetos. | Deveria | M1 |
| RF-PROJ-006 | Vincular projeto a conexões (ex.: servidor de produção), stack Compose e projeto do TaskHub. | Poderia | M3 |
| RF-PROJ-007 | Integração com GitHub/GitLab: PRs abertos, status de CI, issues atribuídas. | Poderia | Futuro |
| RF-PROJ-008 | Descoberta incremental com *watcher* de sistema de arquivos, sem varrer o disco inteiro periodicamente. | Deve | M1 |
| RF-PROJ-009 | Rotinas de projeto ("modo trabalho"): abrir editor, subir containers e conectar túneis com uma ação. | Poderia | Futuro |

## Regras de negócio

- Caminhos são **por dispositivo**: a lista de projetos não sincroniza; sincronizam apenas preferências (favoritos por URL do remoto, tags, editor preferido).
- Projetos dentro do WSL são lidos **de dentro da distro** (`wsl -d X -e git …`) — ler via `\\wsl$` é lento demais. Só para distros em execução (ver [RF-SYS-024](sistema.md#wsl-somente-windows)).
- O status git é calculado com `gix` (gitoxide) ou `git` CLI, com *timeout*; repositórios enormes não podem travar a lista.

## Critérios de aceite

- **Dado** a pasta raiz `C:\dev` com 40 repositórios, **quando** a primeira varredura termina, **então** todos aparecem em menos de 5 s e mudanças posteriores (novo commit) refletem em até 10 s sem nova varredura completa.
- **Dado** um projeto com arquivos modificados, **quando** a lista é exibida, **então** o projeto mostra o indicador de alterações pendentes e a branch atual.

## Perguntas em aberto

- Onde ficam seus projetos e quais editores você usa? → [Q-15](../10-perguntas-em-aberto.md#sistema)
