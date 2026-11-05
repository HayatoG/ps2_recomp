<#
.SYNOPSIS
  Roda um executavel de port e amostra CPU (total e por thread), RAM e GPU a cada 1 s. Generico (nada de um jogo).

.DESCRIPTION
  * Define PS2X_FPS_LOG=<OutDir>\<Tag>.fps.csv (t_s,fps,avg_ms,max_ms,guest_vsync_hz) e as variaveis de -Env
    so no processo filho; PS2X_PAD_SCRIPT vem de -PadScript.
  * Amostra o processo a cada ~1 s: CPU% (tempo de CPU do processo / tempo de parede / -Cores), CPU por thread
    (as -TopThreads mais quentes, em % de UM nucleo), working set e pico, e GPU (contadores do Windows
    '\GPU Engine(*)\Utilization Percentage' filtrados pelo PID; por tipo de engine soma as instancias e usa o
    maior tipo; vazio se os contadores nao existirem).
  * Escreve <OutDir>\<Tag>.cpu.csv, <Tag>.summary.json e <Tag>.out.log (stdout/stderr do filho).
    O resumo traz: vsync/s medio na janela de vsync ACUMULADO (-VsyncWindow, padrao 7530 7640, interpolado), vsync
    medio da corrida, CPU% medio, pico de RAM, GPU% medio e as threads mais quentes.
  * Log proprio em <OutDir>\perf_run_<Tag>.log (timestamp e nivel). Rode a partir do diretorio do jogo
    (OutDir padrao: logs\perf).
  * Para medidas limpas, pare outros trabalhos pesados antes: o resumo registra 'loadavg_hint' (CPU% do
    sistema antes de iniciar) para saber se a maquina estava carregada.

.PARAMETER Exe           Executavel do port.
.PARAMETER ExeArgs       (alias -Args) Argumentos do executavel (array).
.PARAMETER Env           Hashtable (@{PS2X_GS='cpu'}) ou, com 'powershell -File', texto 'PS2X_GS=cpu;PS2X_HEARTBEAT=1'.
.PARAMETER PadScript     Script de entrada do pad (vira PS2X_PAD_SCRIPT).
.PARAMETER TimeoutSec    Mata o processo apos esse tempo (padrao 600).
.PARAMETER OutDir        Pasta de saida (padrao logs\perf).
.PARAMETER Tag           Nome da rodada (prefixo dos arquivos).
.PARAMETER Cores         Threads logicas para o CPU% (padrao: as da maquina).
.PARAMETER TopThreads    Quantas threads quentes listar (padrao 4).
.PARAMETER VsyncWindow   Janela de vsync acumulado (padrao 7530,7640).

.EXAMPLE
  cd ghost_pc; powershell -NoProfile -File ..\tools\perf_run.ps1 -Exe work\...\ps2EntryRunner.exe -Tag novo_cpu `
    -Env @{PS2X_GS='cpu'; PS2X_TARGET_FPS='0'; PS2X_VIDEO_HZ='50'} -PadScript pad_scripts\scene3d.txt -TimeoutSec 400
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Exe,
    [Alias('Args')][string[]]$ExeArgs = @(),
    [object]$Env = @{},
    [string]$PadScript = '',
    [int]$TimeoutSec = 600,
    [string]$OutDir = 'logs\perf',
    [Parameter(Mandatory)][string]$Tag,
    [int]$Cores = [Environment]::ProcessorCount,
    [int]$TopThreads = 4,
    [int[]]$VsyncWindow = @(7530, 7640)
)
$ErrorActionPreference = 'Stop'
[void](New-Item -ItemType Directory -Force $OutDir)
$OutDir = (Resolve-Path $OutDir).Path
$logPath = Join-Path $OutDir "perf_run_$Tag.log"
function Write-Log([string]$Level, [string]$Message) {
    $line = '{0} {1} perf_run: {2}' -f (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss'), $Level, $Message
    Add-Content -Encoding UTF8 -Path $logPath -Value $line
    Write-Host $line
}

function Get-VsyncWindowRate([string]$Csv, [int]$Lo, [int]$Hi) {
    # vsync acumulado = soma de guest_vsync_hz * dt; taxa na janela por interpolacao linear do tempo.
    $rows = @(Import-Csv $Csv)
    if ($rows.Count -lt 2) { return @{ window = $null; mean = $null; total = 0 } }
    $inv = [Globalization.CultureInfo]::InvariantCulture
    $prev = 0.0; $cum = 0.0; $pts = @()
    foreach ($r in $rows) {
        $t = [double]::Parse($r.t_s, $inv); $cum += [double]::Parse($r.guest_vsync_hz, $inv) * ($t - $prev); $prev = $t
        $pts += , @($t, $cum)
    }
    $timeAt = {
        param($target)
        for ($i = 1; $i -lt $pts.Count; $i++) {
            if ($pts[$i][1] -ge $target -and $pts[$i][1] -gt $pts[$i - 1][1]) {
                $f = ($target - $pts[$i - 1][1]) / ($pts[$i][1] - $pts[$i - 1][1])
                return $pts[$i - 1][0] + $f * ($pts[$i][0] - $pts[$i - 1][0])
            }
        }
        return $null
    }
    $t0 = & $timeAt $Lo; $t1 = & $timeAt $Hi
    $win = if ($null -ne $t0 -and $null -ne $t1 -and $t1 -gt $t0) { ($Hi - $Lo) / ($t1 - $t0) } else { $null }
    return @{ window = $win; mean = $cum / $prev; total = $cum }
}

function Get-GpuPercent([int]$ProcessId) {
    try {
        $s = (Get-Counter '\GPU Engine(*)\Utilization Percentage' -SampleInterval 1 -MaxSamples 1 -ErrorAction Stop).CounterSamples
    } catch { return $null }
    $byType = @{}
    foreach ($c in $s) {
        if ($c.InstanceName -match "^pid_${ProcessId}_.*engtype_(.+)$") {
            $byType[$Matches[1]] = [double]($byType[$Matches[1]]) + $c.CookedValue
        }
    }
    if ($byType.Count -eq 0) { return 0.0 }
    return [Math]::Min(100.0, ($byType.Values | Measure-Object -Maximum).Maximum)
}

if (-not (Test-Path $Exe)) { Write-Log 'ERROR' "executavel nao encontrado: $Exe"; exit 2 }
$Exe = (Resolve-Path $Exe).Path
$fpsCsv = Join-Path $OutDir "$Tag.fps.csv"
$cpuCsv = Join-Path $OutDir "$Tag.cpu.csv"
Remove-Item -ErrorAction SilentlyContinue $fpsCsv, $cpuCsv

# Dica de carga antes de iniciar (CPU do sistema em ~1 s).
$hint = $null
try { $hint = [double](Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'").PercentProcessorTime } catch { }  # independe do idioma do Windows

$saved = @{}
$childEnv = @{ PS2X_FPS_LOG = $fpsCsv }
if ($PadScript) { $childEnv['PS2X_PAD_SCRIPT'] = (Resolve-Path $PadScript).Path }
if ($Env -is [string]) {
    $parsed = @{}
    foreach ($pair in ($Env -split ';' | Where-Object { $_ })) { $k, $v = $pair -split '=', 2; $parsed[$k.Trim()] = $v }
    $Env = $parsed
}
foreach ($k in $Env.Keys) { $childEnv[$k] = [string]$Env[$k] }
foreach ($k in $childEnv.Keys) { $saved[$k] = [Environment]::GetEnvironmentVariable($k); [Environment]::SetEnvironmentVariable($k, $childEnv[$k]) }
Write-Log 'INFO' "iniciando $Exe (timeout ${TimeoutSec}s, ${Cores} threads logicas, carga do sistema ao iniciar: $hint%)"
try {
    $startArgs = @{ FilePath = $Exe; PassThru = $true; WorkingDirectory = (Get-Location).Path
        RedirectStandardOutput = (Join-Path $OutDir "$Tag.out.log"); RedirectStandardError = (Join-Path $OutDir "$Tag.err.log") }
    if ($ExeArgs.Count) { $startArgs['ArgumentList'] = $ExeArgs }
    $proc = Start-Process @startArgs
} finally {
    foreach ($k in $saved.Keys) { [Environment]::SetEnvironmentVariable($k, $saved[$k]) }
}

$header = 't_s,cpu_pct,cpu_cores,ws_mb,peak_ws_mb,gpu_pct,' + ((1..$TopThreads | ForEach-Object { "hot${_}_tid,hot${_}_pct" }) -join ',')
Set-Content -Encoding UTF8 -Path $cpuCsv -Value $header
$inv = [Globalization.CultureInfo]::InvariantCulture
$watch = [Diagnostics.Stopwatch]::StartNew()
$lastT = 0.0; $lastCpu = [TimeSpan]::Zero; $lastThreads = @{}
$cpuPcts = @(); $gpuPcts = @(); $peakWs = 0.0; $threadTotals = @{}; $timedOut = $false

while (-not $proc.HasExited) {
    if ($watch.Elapsed.TotalSeconds -ge $TimeoutSec) { $timedOut = $true; break }
    $gpu = Get-GpuPercent $proc.Id   # bloqueia ~1 s quando ha contadores
    if ($null -eq $gpu) { Start-Sleep -Seconds 1 }
    try {
        $proc.Refresh()
        $now = $watch.Elapsed.TotalSeconds; $dt = $now - $lastT
        $cpu = $proc.TotalProcessorTime
        $used = ($cpu - $lastCpu).TotalSeconds / $dt
        if ($used -lt 0 -or $proc.HasExited) { break }   # processo encerrando: o tempo de CPU "volta" e gera amostra negativa
        $threads = @{}
        foreach ($th in $proc.Threads) { try { $threads[$th.Id] = $th.TotalProcessorTime } catch { } }
        $deltas = foreach ($id in $threads.Keys) {
            $prev = if ($lastThreads.ContainsKey($id)) { $lastThreads[$id] } else { [TimeSpan]::Zero }
            [pscustomobject]@{ Id = $id; Pct = 100.0 * ($threads[$id] - $prev).TotalSeconds / $dt }
        }
        $hot = @($deltas | Sort-Object Pct -Descending | Select-Object -First $TopThreads)
        foreach ($h in $hot) { $threadTotals[$h.Id] = [double]($threadTotals[$h.Id]) + $h.Pct * $dt }
        $ws = $proc.WorkingSet64 / 1MB; $peak = $proc.PeakWorkingSet64 / 1MB
        if ($peak -gt $peakWs) { $peakWs = $peak }
        $pct = 100.0 * $used / $Cores
        $cpuPcts += $pct; if ($null -ne $gpu) { $gpuPcts += $gpu }
        $cells = @($now.ToString('F3', $inv), $pct.ToString('F1', $inv), $used.ToString('F2', $inv), $ws.ToString('F0', $inv),
                   $peak.ToString('F0', $inv), $(if ($null -ne $gpu) { $gpu.ToString('F1', $inv) } else { '' }))
        for ($i = 0; $i -lt $TopThreads; $i++) {
            if ($i -lt $hot.Count) { $cells += $hot[$i].Id; $cells += $hot[$i].Pct.ToString('F1', $inv) } else { $cells += ''; $cells += '' }
        }
        Add-Content -Encoding UTF8 -Path $cpuCsv -Value ($cells -join ',')
        $lastT = $now; $lastCpu = $cpu; $lastThreads = $threads
    } catch { if ($proc.HasExited) { break } else { Write-Log 'WARN' "amostra falhou: $($_.Exception.Message)" } }
}
if ($timedOut) {
    Write-Log 'INFO' "timeout de ${TimeoutSec}s: encerrando o processo"
    & taskkill /PID $proc.Id /T /F | Out-Null
}
$proc.WaitForExit(5000) | Out-Null
if (-not $gpuPcts.Count) { Write-Log 'WARN' 'sem amostras de GPU (contadores GPU Engine indisponiveis); gpu_pct vazio' }
$elapsed = [Math]::Round($watch.Elapsed.TotalSeconds, 1)

$vs = @{ window = $null; mean = $null; total = 0 }
if (Test-Path $fpsCsv) { $vs = Get-VsyncWindowRate $fpsCsv $VsyncWindow[0] $VsyncWindow[1] }
$hotList = @($threadTotals.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First $TopThreads |
    ForEach-Object { [ordered]@{ tid = $_.Key; mean_pct_of_one_core = [Math]::Round($_.Value / [Math]::Max($lastT, 1), 1) } })
function Round-Or-Null($values) { if ($values.Count) { [Math]::Round(($values | Measure-Object -Average).Average, 2) } else { $null } }
$summary = [ordered]@{
    tag = $Tag; exe_name = (Split-Path $Exe -Leaf); started = (Get-Date).AddSeconds(-$elapsed).ToString('s')
    duration_s = $elapsed; timed_out = $timedOut; exit_code = $(if ($proc.HasExited -and -not $timedOut) { $proc.ExitCode } else { $null })
    logical_threads = $Cores; loadavg_hint_pct = $hint
    vsync_window = $VsyncWindow; vsync_per_s_window = $(if ($null -ne $vs.window) { [Math]::Round($vs.window, 2) } else { $null })
    vsync_per_s_run = $(if ($null -ne $vs.mean) { [Math]::Round($vs.mean, 2) } else { $null }); vsync_total = [Math]::Round($vs.total, 0)
    cpu_pct_mean = (Round-Or-Null $cpuPcts); cpu_cores_mean = $(if ($cpuPcts.Count) { [Math]::Round((Round-Or-Null $cpuPcts) * $Cores / 100, 2) } else { $null })
    peak_ram_mb = [Math]::Round($peakWs, 0); gpu_pct_mean = (Round-Or-Null $gpuPcts); samples = $cpuPcts.Count
    hottest_threads = $hotList
    note = 'vsync_per_s_window e null se a corrida nao chegou ao fim da janela; carga do sistema alta invalida comparacoes.'
}
$summary | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 (Join-Path $OutDir "$Tag.summary.json")
Write-Log 'INFO' ("pronto: {0}s, vsync/s janela={1}, CPU medio={2}%, pico RAM={3} MB, GPU medio={4}%" -f $elapsed, $summary.vsync_per_s_window, $summary.cpu_pct_mean, $summary.peak_ram_mb, $summary.gpu_pct_mean)
