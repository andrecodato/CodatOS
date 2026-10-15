---
tags: [modulo]
status: planejamento
marco: M1
spec: docs/modulos/projetos.md
---
# Projetos

"Em que eu estava trabalhando?" — repositórios descobertos em pastas raiz (Windows e dentro do WSL) + recentes do VS Code/Cursor/JetBrains, com branch, alterações pendentes e ações rápidas.

**Conceitos:** [[Coletor]] · [[Local-first]] (caminhos são por dispositivo)

## Fios soltos
- Git dentro do WSL: rodar **dentro** da distro; `\\wsl$` é lento demais → [[WSL por dentro]]
- `gix` (gitoxide) para status sem processo externo, com timeout.
- Watcher incremental (crate `notify`), sem varrer o disco todo.

## Ideias
- Rotina "modo trabalho" por projeto (editor + compose + túnel).
- PRs/CI do GitHub por projeto.
- Relatório semanal de horas por projeto via commits.
