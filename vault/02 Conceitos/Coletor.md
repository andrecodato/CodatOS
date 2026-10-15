---
tags: [conceito, arquitetura]
aliases: [Coletores, Collector]
---
# Coletor

Componente do [[Agente local]] que lê uma fonte de dados: `system.cpu`, `disks`, `wsl.distros`, `docker.containers`, `projects`…

- **Sob demanda**: roda só se alguém assina o tópico ou há alerta configurado.
- **Isolado**: timeout, backoff, erro próprio — um coletor travado não derruba os outros.
- **Cache + histórico**: último snapshot em memória e ring buffer de 1 h.
- **Deltas**: a UI recebe snapshot inicial e depois só mudanças.

Armadilhas conhecidas: [[WSL por dentro]] (não ligar distro parada!) · [[Docker em múltiplos engines]] (deduplicar) · [[Temperatura no Windows]].

📄 `docs/02-arquitetura.md#coletores` · Módulo: [[Monitor do sistema]]
