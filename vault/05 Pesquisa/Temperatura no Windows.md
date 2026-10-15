---
tags: [pesquisa]
status: decidida
relacionado: Q-13
---
# Temperatura (e watts) no Windows

**Pergunta:** como mostrar temperatura de CPU e consumo em watts no Windows sem exigir admin?

## O que se sabe
- O Windows **não** expõe temperatura de CPU de forma padronizada sem driver de kernel.
- WMI `MSAcpi_ThermalZoneTemperature` (namespace `root\WMI`): exige admin e costuma devolver zonas ACPI, não o *package* da CPU — muitas máquinas retornam valor fixo ou nada.
- **GPU NVIDIA**: NVML funciona sem admin (temperatura, uso, potência). ✅
- **AMD**: SDK ADLX no Windows. **Intel GPU**: limitado.
- **LibreHardwareMonitor** (open source, MPL-2.0) lê sensores via driver. O driver antigo **WinRing0** tem vulnerabilidade conhecida (CVE-2020-14979) e passou a ser bloqueado pelo Microsoft Defender; o LHM migrou para o driver **PawnIO**. _(confirmar versões antes de integrar)_
- O LHM expõe dados por **servidor web local (JSON)** e por **WMI** quando está rodando.

## Linux e macOS (para comparação)
- Linux: hwmon via `sysinfo` ✅. Watts via RAPL (`/sys/class/powercap/intel-rapl*/energy_uj`) — **legível só por root** desde a mitigação do ataque PLATYPUS (CVE-2020-8694).
- macOS: temperaturas via SMC/IOKit (parcial, sem root); `powermetrics` exige `sudo`.

## Opções para o CodatOS
1. **Sem privilégio (padrão):** GPU via NVML + o que existir; demais campos "indisponível".
2. **Integrar com LHM instalado pelo usuário:** ler o JSON do servidor web dele. Simples, sem driver nosso.
3. **Helper privilegiado próprio** (`codat-helper`) usando PawnIO: melhor UX, mais responsabilidade de segurança.

**Decidido (Q-13, 2026-10-15):** 1 + 2 no M1; 3 só depois da 1.0, se houver demanda.

Ver [[Monitor do sistema]] · [[Coletor]]
