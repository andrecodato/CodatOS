---
tags: [pesquisa]
status: aberta
---
# Docker em múltiplos engines

Pedido original: "dockers rodando (dentro dos WSL também)". Um PC pode ter vários daemons ao mesmo tempo.

## Onde procurar
| SO      | Endpoints comuns                                                                                                                                  |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows | `npipe:////./pipe/docker_engine`, `npipe:////./pipe/dockerDesktopLinuxEngine` (contexto `desktop-linux`); `docker context ls` lista os conhecidos |
| WSL     | `dockerd` nativo na distro (`/var/run/docker.sock` **dentro** dela) — acesso via `wsl -d X -e docker ...`                                         |
| macOS   | Docker Desktop (`~/.docker/run/docker.sock`), Colima (`~/.colima/default/docker.sock`), OrbStack (`~/.orbstack/run/docker.sock`), Podman machine  |
| Linux   | `/var/run/docker.sock`, rootless `$XDG_RUNTIME_DIR/docker.sock`, Podman (`podman.sock`, API compatível)                                           |

## Deduplicação
- Com a integração WSL do Docker Desktop ligada, o **mesmo engine** aparece no Windows e dentro das distros. Identificar pelo campo `ID` de `docker info` (ou `GET /info`) e mostrar uma vez só.

## Coleta
- Host: crate **bollard** (API HTTP do Docker por pipe/socket).
- Dentro do WSL: CLI `docker ps --format '{{json .}}'` e `docker stats --no-stream --format '{{json .}}'` (somente distros em execução — ver [[WSL por dentro]]).
- Agrupar por label `com.docker.compose.project`.

## Ações e logs
- start/stop/restart/exec com permissão `system.control` → [[Grupos e permissões]]
- Logs só ao vivo, nunca persistidos (R-01/R-06).

Ver [[Monitor do sistema]] · [[Coletor]]
