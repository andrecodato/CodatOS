<#
.SYNOPSIS
    Mede a RAM (e a CPU) do CodatOS em repouso: janela fechada, app na bandeja.

.DESCRIPTION
    Abre o app, espera a janela principal aparecer, fecha a janela (o M0 destrói a
    WebView ao fechar e o app fica na bandeja), espera alguns segundos e soma o
    working set e os private bytes do processo codatos e de todos os descendentes
    (inclusive os msedgewebview2, se algum sobrar). No fim, encerra o app.

    Alvo: RNF-DES-01 (docs/01-requisitos.md) — em repouso, RAM < 80 MB e CPU
    média < 1%. Resultado registrado em docs/benchmarks/m0.md.

    Compatível com PowerShell 7 e Windows PowerShell 5.1.

.PARAMETER ExePath
    Caminho do codatos.exe. Padrão: o binário gerado por `just build-win`.

.PARAMETER WaitSeconds
    Quanto esperar, depois de fechar a janela, antes de medir. Padrão: 30.

.PARAMETER CpuSampleSeconds
    Janela de amostragem da CPU média, logo após a espera. 0 desliga. Padrão: 10.

.PARAMETER Autostart
    Inicia com `--autostart` (já minimizado na bandeja) em vez de abrir e fechar
    a janela.

.PARAMETER Json
    Imprime o resultado em JSON em vez de tabela.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\measure-idle.ps1

.EXAMPLE
    pwsh -File scripts\measure-idle.ps1 -ExePath "$env:LOCALAPPDATA\CodatOS\codatos.exe" -Autostart -Json
#>
[CmdletBinding()]
param(
    # Vazio = o binário do `just build-win` (resolvido abaixo; no Windows PowerShell
    # 5.1 o $PSScriptRoot pode vir vazio dentro do param()).
    [string]$ExePath = '',
    [ValidateRange(0, 3600)]
    [int]$WaitSeconds = 30,
    [ValidateRange(0, 600)]
    [int]$CpuSampleSeconds = 10,
    [switch]$Autostart,
    [switch]$Json
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

if (-not $ExePath) {
    # Raiz do repo: pela pasta do script; se o PowerShell não informar, pela pasta atual
    # (o `just measure-idle` roda na raiz).
    $scriptDir = $PSScriptRoot
    if (-not $scriptDir -and $MyInvocation.MyCommand.Path) {
        $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    }
    $repoRoot = if ($scriptDir) { Split-Path -Parent $scriptDir } else { (Get-Location).ProviderPath }
    $ExePath = Join-Path $repoRoot 'target\x86_64-pc-windows-msvc\release\codatos.exe'
}

# GUID do WebView2 Runtime (Evergreen) no EdgeUpdate.
$WebView2Guid = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$MB = 1MB

# --- Funções auxiliares -------------------------------------------------------

# Lê a versão instalada do WebView2 Runtime. Instalação por máquina fica em
# WOW6432Node; por usuário, em HKCU. Devolve $null se não achar.
function Get-WebView2Version {
    $keys = @(
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$WebView2Guid",
        "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$WebView2Guid",
        "HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$WebView2Guid"
    )
    foreach ($key in $keys) {
        $item = Get-ItemProperty -Path $key -Name 'pv' -ErrorAction SilentlyContinue
        if ($item -and $item.pv -and $item.pv -ne '0.0.0.0') {
            return [string]$item.pv
        }
    }
    return $null
}

# Dados da máquina para registrar junto com os números.
function Get-MachineInfo {
    $os = Get-CimInstance -ClassName Win32_OperatingSystem
    $cpu = Get-CimInstance -ClassName Win32_Processor | Select-Object -First 1
    [pscustomobject]@{
        Os          = "$($os.Caption) $($os.Version) (build $($os.BuildNumber))"
        Cpu         = ([string]$cpu.Name).Trim()
        LogicalCpus = [Environment]::ProcessorCount
        RamTotalGB  = [math]::Round(([double]$os.TotalVisibleMemorySize * 1KB) / 1GB, 1)
        WebView2    = Get-WebView2Version
        PowerShell  = $PSVersionTable.PSVersion.ToString()
    }
}

# Devolve o PID raiz e todos os descendentes, percorrendo ParentProcessId do
# Win32_Process. É assim que os msedgewebview2 (filhos/netos do app) entram.
function Get-ProcessTreeIds {
    param([int]$RootId)

    $all = Get-CimInstance -ClassName Win32_Process -Property ProcessId, ParentProcessId
    $children = @{}
    foreach ($p in $all) {
        $parent = [int]$p.ParentProcessId
        if (-not $children.ContainsKey($parent)) {
            $children[$parent] = New-Object System.Collections.Generic.List[int]
        }
        $children[$parent].Add([int]$p.ProcessId)
    }

    $result = New-Object System.Collections.Generic.List[int]
    $queue = New-Object System.Collections.Generic.Queue[int]
    $queue.Enqueue($RootId)
    while ($queue.Count -gt 0) {
        $id = $queue.Dequeue()
        if ($result.Contains($id)) { continue }  # proteção contra PIDs reciclados
        $result.Add($id)
        if ($children.ContainsKey($id)) {
            foreach ($child in $children[$id]) { $queue.Enqueue($child) }
        }
    }
    return , $result.ToArray()
}

# Processos vivos da árvore (ignora os que morreram no meio do caminho).
function Get-ProcessTree {
    param([int]$RootId)
    foreach ($id in (Get-ProcessTreeIds -RootId $RootId)) {
        Get-Process -Id $id -ErrorAction SilentlyContinue
    }
}

# Soma do tempo de CPU (ms) da árvore inteira.
function Get-TreeCpuMs {
    param([int]$RootId)
    $total = 0.0
    foreach ($p in (Get-ProcessTree -RootId $RootId)) {
        try { $total += $p.TotalProcessorTime.TotalMilliseconds } catch { }
    }
    return $total
}

# Encerra o app e o que sobrar da árvore.
function Stop-ProcessTree {
    param([int]$RootId)
    $ids = Get-ProcessTreeIds -RootId $RootId
    foreach ($id in $ids) {
        Stop-Process -Id $id -Force -ErrorAction SilentlyContinue
    }
}

# --- Preparação ---------------------------------------------------------------

if (-not (Test-Path -LiteralPath $ExePath -PathType Leaf)) {
    throw "codatos.exe não encontrado em '$ExePath'. Rode 'just build-win' ou informe -ExePath."
}
$ExePath = (Resolve-Path -LiteralPath $ExePath).ProviderPath
$processName = [IO.Path]::GetFileNameWithoutExtension($ExePath)

# Com instância única, um app já aberto faria o novo processo sair na hora.
$running = @(Get-Process -Name $processName -ErrorAction SilentlyContinue)
if ($running.Count -gt 0) {
    throw "Já existe '$processName' rodando (PID $($running[0].Id)). Saia pela bandeja e rode de novo."
}

$versionInfo = (Get-Item -LiteralPath $ExePath).VersionInfo
$appVersion = $versionInfo.ProductVersion
if (-not $appVersion) { $appVersion = $versionInfo.FileVersion }

$machine = Get-MachineInfo

# --- Execução -----------------------------------------------------------------

$mode = if ($Autostart) { 'autostart' } else { 'janela fechada' }
Write-Host "Iniciando $ExePath ($mode)..."

$startArgs = @{ FilePath = $ExePath; PassThru = $true }
if ($Autostart) { $startArgs['ArgumentList'] = '--autostart' }
$proc = Start-Process @startArgs

try {
    if (-not $Autostart) {
        # Espera a janela principal existir (até 30 s) e a fecha com WM_CLOSE,
        # o mesmo caminho do botão X. O app deve continuar vivo na bandeja.
        $deadline = (Get-Date).AddSeconds(30)
        while ($true) {
            if ($proc.HasExited) { throw "O app saiu antes de abrir a janela (código $($proc.ExitCode))." }
            $proc.Refresh()
            if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { break }
            if ((Get-Date) -gt $deadline) { throw 'A janela principal não apareceu em 30 s.' }
            Start-Sleep -Milliseconds 250
        }
        # Dá um instante para a UI terminar de carregar antes de fechar.
        Start-Sleep -Seconds 2
        Write-Host 'Fechando a janela principal...'
        [void]$proc.CloseMainWindow()
    }

    Write-Host "Aguardando $WaitSeconds s em repouso..."
    Start-Sleep -Seconds $WaitSeconds

    if ($proc.HasExited) {
        throw "O app encerrou ao fechar a janela (código $($proc.ExitCode)). Ele deveria ficar na bandeja."
    }

    # CPU média da árvore durante a janela de amostragem.
    $cpuPercent = $null
    if ($CpuSampleSeconds -gt 0) {
        Write-Host "Amostrando CPU por $CpuSampleSeconds s..."
        $cpu0 = Get-TreeCpuMs -RootId $proc.Id
        $t0 = [Diagnostics.Stopwatch]::StartNew()
        Start-Sleep -Seconds $CpuSampleSeconds
        $cpu1 = Get-TreeCpuMs -RootId $proc.Id
        $elapsedMs = $t0.Elapsed.TotalMilliseconds
        $cpuPercent = [math]::Round((($cpu1 - $cpu0) / ($elapsedMs * [Environment]::ProcessorCount)) * 100, 2)
    }

    # Memória de cada processo da árvore.
    $rows = @(
        foreach ($p in (Get-ProcessTree -RootId $proc.Id)) {
            [pscustomobject]@{
                Pid          = $p.Id
                Name         = $p.ProcessName
                WorkingSetMB = [math]::Round($p.WorkingSet64 / $MB, 1)
                PrivateMB    = [math]::Round($p.PrivateMemorySize64 / $MB, 1)
            }
        }
    )

    $wsTotal = ($rows | Measure-Object -Property WorkingSetMB -Sum).Sum
    $privTotal = ($rows | Measure-Object -Property PrivateMB -Sum).Sum
    $webviews = @($rows | Where-Object { $_.Name -like 'msedgewebview2*' }).Count

    $result = [pscustomobject]@{
        Date              = (Get-Date).ToString('yyyy-MM-dd HH:mm')
        AppVersion        = $appVersion
        ExePath           = $ExePath
        Mode              = $mode
        WaitSeconds       = $WaitSeconds
        CpuSampleSeconds  = $CpuSampleSeconds
        Machine           = $machine
        Processes         = $rows
        WebViewProcesses  = $webviews
        TotalWorkingSetMB = [math]::Round($wsTotal, 1)
        TotalPrivateMB    = [math]::Round($privTotal, 1)
        CpuPercent        = $cpuPercent
        # RNF-DES-01: RAM < 80 MB em repouso (usamos o maior dos dois totais).
        MeetsRnfDes01Ram  = ([math]::Max($wsTotal, $privTotal) -lt 80)
    }

    if ($Json) {
        $result | ConvertTo-Json -Depth 4
    }
    else {
        Write-Host ''
        Write-Host "CodatOS $appVersion · $mode · espera $WaitSeconds s"
        Write-Host "SO: $($machine.Os)"
        Write-Host "CPU: $($machine.Cpu) ($($machine.LogicalCpus) lógicos) · RAM: $($machine.RamTotalGB) GB"
        Write-Host "WebView2 Runtime: $($machine.WebView2)"
        $rows | Sort-Object -Property PrivateMB -Descending | Format-Table -AutoSize | Out-String | Write-Host
        Write-Host ("Total working set: {0:N1} MB" -f $wsTotal)
        Write-Host ("Total private bytes: {0:N1} MB" -f $privTotal)
        Write-Host "Processos msedgewebview2 vivos: $webviews"
        if ($null -ne $cpuPercent) { Write-Host ("CPU média ({0} s): {1:N2}%" -f $CpuSampleSeconds, $cpuPercent) }
        $verdict = if ($result.MeetsRnfDes01Ram) { 'atende' } else { 'NÃO atende' }
        Write-Host "RNF-DES-01 (RAM < 80 MB): $verdict"
    }
}
finally {
    # Sempre encerra o app, mesmo se a medição falhar.
    if (-not $proc.HasExited) { Stop-ProcessTree -RootId $proc.Id }
}
