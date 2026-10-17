# Módulo · Monitor do sistema

> **Prefixo:** `RF-SYS` · **Marco principal:** M1 · **Status:** Rascunho
> ↑ [Requisitos](../01-requisitos.md) · Relacionados: [Dashboard](dashboard.md) · [Conexões](conexoes.md) (shells de WSL/Docker) · [Arquitetura → coletores](../02-arquitetura.md#coletores)

## Objetivo

Mostrar — e permitir controlar — o que está acontecendo no computador: hardware, discos, distros WSL e containers Docker, inclusive os que rodam **dentro** das distros WSL. Dados são coletados localmente pelo núcleo Rust e nunca saem do PC, exceto quando exibidos ao vivo para um cliente remoto autorizado.

## Telas

- Widgets no [dashboard](dashboard.md#catálogo-de-widgets): Uso do PC, Discos, WSL, Docker, Rede, Energia.
- Página **Sistema** (via "Ver todos" ou menu): abas Visão geral · Processos · Discos · WSL · Docker.

## Requisitos funcionais

### Hardware

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-SYS-001 | CPU: modelo, uso total e por núcleo, frequência e temperatura (quando disponível). | Deve | M1 |
| RF-SYS-002 | Memória: usada/total, cache, swap/arquivo de paginação. | Deve | M1 |
| RF-SYS-003 | GPU: uso, VRAM, temperatura e potência (NVIDIA via NVML; AMD/Intel/Apple conforme disponibilidade). | Deveria | M1 |
| RF-SYS-004 | Energia: potência estimada do sistema (CPU package + GPU) e bateria (nível, carregando/descarregando, tempo restante). | Deveria | M3 |
| RF-SYS-005 | Rede: taxa de upload/download por interface, IP local; IP público só com opt-in. | Deveria | M1 |
| RF-SYS-006 | Sistema: SO, versão, hostname, uptime. | Deve | M1 |
| RF-SYS-007 | Processos: top por CPU/RAM, busca, encerrar processo (confirmação + permissão `system.control`). | Poderia | M3 |
| RF-SYS-008 | Histórico em memória (última hora, ring buffer) para gráficos; persistência opcional local. | Deveria | M1 |

### Discos

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-SYS-010 | Listar volumes: letra/ponto de montagem, rótulo, tipo (NVMe/SSD/HDD/removível/rede), sistema de arquivos, usado/total/%. | Deve | M1 |
| RF-SYS-011 | Alerta configurável de espaço livre. | Deveria | M3 |
| RF-SYS-012 | Saúde SMART (via `smartctl`, se instalado). | Poderia | Futuro |
| RF-SYS-013 | "Para onde foi meu espaço": maiores consumidores conhecidos (VHDX do WSL, imagens/volumes Docker, `node_modules`, caches). | Poderia | Futuro |

### WSL (somente Windows)

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-SYS-020 | Listar distros: nome, estado (em execução/parada), versão WSL (1/2), distro padrão. | Deve | M1 |
| RF-SYS-021 | Tamanho de cada instância: tamanho do arquivo `ext4.vhdx` no host (todas) e uso interno via `df` (somente em execução). | Deve | M1 |
| RF-SYS-022 | Para distros em execução: tempo ligado, CPU e RAM (da VM do WSL e, quando possível, por distro). | Deve | M1 |
| RF-SYS-023 | Ações: iniciar, encerrar (`wsl -t`), desligar o WSL (`wsl --shutdown`), abrir terminal, definir padrão. | Deveria | M1 |
| RF-SYS-024 | **A coleta nunca pode iniciar uma distro parada** (chamar `wsl -d X` liga a distro). | Deve | M1 |
| RF-SYS-025 | Assistente para compactar o VHDX (liberar espaço). | Poderia | Futuro |

### Docker

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-SYS-030 | Detectar engines: host (Docker Desktop, dockerd nativo, Colima/OrbStack no macOS, Podman com API compatível) e `dockerd` dentro de cada distro WSL **em execução**. | Deve | M1 |
| RF-SYS-031 | Listar containers por engine: nome, imagem, status, tempo ligado, portas, CPU/RAM; agrupar por projeto Compose. | Deve | M1 |
| RF-SYS-032 | Deduplicar engines: o Docker Desktop aparece tanto no host quanto na distro `docker-desktop` — identificar pelo ID do engine (`docker info`). | Deve | M1 |
| RF-SYS-033 | Ações: iniciar, parar, reiniciar, ver logs ao vivo (sem persistir), abrir shell (`exec`) — com permissão `system.control`. | Deveria | M2 |
| RF-SYS-034 | Uso de disco do Docker (imagens, volumes, cache de build) e limpeza guiada (`prune`). | Poderia | M3 |
| RF-SYS-035 | Stacks Compose: subir/derrubar projetos conhecidos. | Poderia | Futuro |

## Fontes de dados por plataforma

| Dado | Windows | Linux | macOS |
|---|---|---|---|
| CPU, RAM, rede, discos, processos | `sysinfo` | `sysinfo` | `sysinfo` |
| Temperatura CPU | ⚠️ ver abaixo | hwmon (`sysinfo`) | SMC (`sysinfo`, parcial) |
| GPU NVIDIA | NVML | NVML | — |
| GPU AMD/Intel | LibreHardwareMonitor (servidor web), quando está rodando | sysfs `amdgpu` (`i915` depois) | IOKit (Apple Silicon, parcial) |
| Potência CPU | ⚠️ ver abaixo | RAPL (`/sys/class/powercap`) | ⚠️ `powermetrics` exige root |
| Bateria | API do SO (`battery`/`starship-battery`) | idem | idem |
| WSL | `wsl.exe` + Registro | — | — |
| Docker | API via named pipe | socket Unix | socket Unix (Docker Desktop/Colima/OrbStack) |

### Temperatura e energia no Windows

O Windows não expõe temperatura de CPU nem potência de forma confiável sem driver de kernel. As opções:

1. **Sem privilégio (padrão)** — mostrar o que existe: GPU via NVML, zonas térmicas ACPI quando disponíveis; os demais campos aparecem como "indisponível — ativar sensores avançados".
2. **Sensores avançados (opt-in)** — integrar com o **LibreHardwareMonitor** (que usa o driver PawnIO em vez do antigo WinRing0, hoje bloqueado pelo Defender), lendo do servidor web/WMI dele, ou instalar um *helper* privilegiado mínimo do CodatOS.

**Decidido em 2026-10-15 ([Q-13](../10-perguntas-em-aberto.md#sistema)):** opção 1 por padrão + leitura do LibreHardwareMonitor quando ele estiver rodando (servidor web local, `http://127.0.0.1:8085/data.json`). Sem o LHM, o widget mostra "temperatura indisponível" com um link de como ativar. Desde 2026-10-17 o LHM também é a fonte das GPUs AMD e Intel no Windows (uso, VRAM, temperatura e potência); sem ele, o widget de GPU pede para abri-lo. Nenhum driver ou serviço privilegiado próprio na 1.0. Pesquisa no vault: `vault/05 Pesquisa/Temperatura no Windows.md`.

### Detalhes de implementação do WSL

- `wsl.exe --list --verbose` devolve texto em **UTF-16LE** — decodificar corretamente.
- Caminho do disco: `HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss\{GUID}` → `BasePath` + `DistributionName`; o tamanho é o do arquivo `ext4.vhdx`.
- Só executar comandos dentro de distros que já estão `Running` (RF-SYS-024).
- RAM/CPU da VM: processo `vmmem`/`vmmemWSL` no host; dentro da distro, `/proc/meminfo` e `/proc/loadavg`.
- Docker dentro do WSL: `wsl -d <distro> -e docker ps --format json` (ou o socket da distro), somente se a distro estiver em execução e tiver `docker` instalado.

## Regras de negócio

- Ações que alteram o sistema (encerrar distro, parar container, matar processo) exigem confirmação e a permissão `system.control` (ver [Multitenancy](multitenancy.md#catálogo-de-permissões)); via acesso remoto, geram notificação no PC.
- Logs de containers são exibidos ao vivo e **nunca persistidos** pelo CodatOS (R-01/R-06).
- Coletores têm *timeout* e *backoff*; um comando travado (ex.: WSL não responde) não bloqueia os demais.

## Critérios de aceite

- **Dado** duas distros (Ubuntu em execução, Debian parada), **quando** o widget WSL atualiza por 10 minutos, **então** o Debian continua parado e mostra apenas o tamanho do VHDX.
- **Dado** o Docker Desktop com integração WSL ligada no Ubuntu, **quando** os containers são listados, **então** cada container aparece uma única vez.
- **Dado** um PC sem GPU NVIDIA nem LibreHardwareMonitor aberto, **quando** o widget Uso do PC é exibido, **então** o anel de GPU mostra "indisponível" (ou é ocultado, conforme configuração) sem erro.

## Perguntas em aberto

- ✅ Temperatura no Windows: LibreHardwareMonitor, sem driver próprio → [Q-13](../10-perguntas-em-aberto.md#sistema)
- Até onde vai o controle (desligar PC, matar processos)? → [Q-14](../10-perguntas-em-aberto.md#sistema)
