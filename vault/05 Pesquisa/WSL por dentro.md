---
tags: [pesquisa]
status: aberta
---
# WSL por dentro

Notas para o [[Coletor]] de WSL ([[Monitor do sistema]]).

## Listar distros
- `wsl.exe --list --verbose` → nome, estado, versão. A saída é **UTF-16LE**; definir `WSL_UTF8=1` no ambiente do processo faz o `wsl.exe` responder em UTF-8.
- `wsl.exe --list --running` → só as que estão ligadas.
- Registro: `HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss\{GUID}` com `DistributionName`, `BasePath`, `Version`; a distro padrão está em `Lxss\DefaultDistribution`.

## Tamanho
- Disco da distro: `<BasePath>\ext4.vhdx` (o `BasePath` pode vir com prefixo `\\?\`). Tamanho do arquivo = espaço ocupado no host.
- Uso interno (só se em execução): `wsl -d X -e df -B1 /`.
- VHDX cresce e **não encolhe sozinho** → compactação (`Optimize-VHD` com Hyper-V, `diskpart compact vdisk`) ou VHD esparso (`wsl --manage X --set-sparse true` nas versões recentes). _(confirmar comandos na versão alvo)_

## ⚠️ Armadilha principal
- **Qualquer `wsl -d X ...` liga a distro se ela estiver parada.** O coletor só pode executar comandos dentro de distros que já estão `Running` (RF-SYS-024).

## CPU e RAM
- A VM do WSL 2 aparece no host como processo `vmmem` / `vmmemWSL` (memória e CPU agregadas de todas as distros).
- Por distro: `/proc/meminfo`, `/proc/loadavg`, `/proc/uptime` (o uptime é da VM, compartilhada).

## Ações
- Encerrar: `wsl -t X` · Desligar tudo: `wsl --shutdown` · Padrão: `wsl -s X` · Iniciar: `wsl -d X -e true` (ou abrir terminal).

## Docker Desktop
- Cria a distro `docker-desktop` (versões antigas também tinham `docker-desktop-data`). Ela deve ser exibida como "infra do Docker", não como distro do usuário → [[Docker em múltiplos engines]]

## Projetos dentro do WSL
- Ler via `\\wsl$\` (9P) é lento; rodar `git`/`find` **dentro** da distro → [[Projetos]].
