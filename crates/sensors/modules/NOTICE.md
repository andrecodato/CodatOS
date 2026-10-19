# Módulos do PawnIO

`AMDFamily17.bin` é o módulo compilado e assinado do projeto
[PawnIO.Modules](https://github.com/namazso/PawnIO.Modules) (código-fonte em
`AMDFamily17.p`), © namazso, sob a **GNU Lesser General Public License 2.1 ou posterior**
(LGPL-2.1-or-later). O arquivo é distribuído sem alteração, como dado embutido no
binário do CodatOS, e só é usado pelo driver PawnIO, que o carrega e confere a assinatura.

- Origem: `LibreHardwareMonitorLib/Resources/PawnIo/AMDFamily17.bin` no repositório do
  [LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor),
  que distribui os mesmos módulos assinados.
- Tamanho: 10652 bytes.
- SHA-256: `dae74615761b78bdf064dfb3e136252ddcc6fc727d88f14738d0e5800d427a91`.

Para trocar o módulo (versão nova, correção), basta substituir o arquivo por outro
compilado do código-fonte acima e assinado pelo autor do PawnIO; o driver recusa módulos
sem assinatura válida.

O driver PawnIO em si (GPL-2.0, com exceção para programas que só falam com ele por
IOCTL) não é distribuído com o CodatOS: o app baixa o instalador oficial
(`namazso/PawnIO.Setup` 2.2.0) quando o usuário ativa a temperatura da CPU.
