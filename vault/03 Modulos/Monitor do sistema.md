---
tags: [modulo]
status: planejamento
marco: M1
spec: docs/modulos/sistema.md
---
# Monitor do sistema

CPU/RAM/GPU/temperatura/energia, discos, distros WSL (estado e tamanho) e containers Docker — inclusive **dentro** das distros WSL. Com ações: iniciar/encerrar distro, parar/reiniciar container.

**Conceitos:** [[Coletor]] · [[Grupos e permissões]] (`system.control`)

## Fios soltos
- Temperatura e watts no Windows precisam de driver → [[Temperatura no Windows]]
- Nunca ligar distro parada ao coletar → [[WSL por dentro]]
- Deduplicar Docker Desktop × distro `docker-desktop` → [[Docker em múltiplos engines]]
- Logs de container: só ao vivo, nunca persistidos (R-01/R-06).

## Ideias
- "Para onde foi meu espaço?" (VHDX, imagens Docker, node_modules).
- Compactar VHDX guiado.
- SMART dos discos.
