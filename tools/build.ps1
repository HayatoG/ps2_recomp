<#
.SYNOPSIS
  Build generico (ninja do Visual Studio + vcvars) com lock, log e verificacao do rastreio de dependencias.

.DESCRIPTION
  * Usa SEMPRE o ninja do VS (1.12.x). Misturar com outro ninja faz cada um descartar o .ninja_log
    do outro ("starting over") e forca rebuild total.
  * Pega o lock <GameDir>\work\BUILD.lock.d (mkdir atomico), com espera e timeout; remove no finally.
  * Grava <GameDir>\logs\build_<carimbo>.log (timestamp por linha de status) e imprime o tempo.
  * Rastreio de dependencias: guarda em <BuildDir>\.build_ps1_state o tamanho/linhas do .ninja_log e
    do .ninja_deps. Se o log encolheu muito ou o .ninja_deps sumiu, AVISA (um header alterado pode
    nao recompilar unidades sem entrada no log). -Recompact roda `ninja -t recompact` apos o build
    (mantem log/deps compactos). -Explain liga `-d explain` (diz por que cada unidade refaz).

.PARAMETER GameDir   Diretorio do jogo (padrao: diretorio corrente), ex.: ghost_pc
.PARAMETER BuildDir  Pasta do build (absoluta ou relativa a GameDir). Padrao: work\ps2recomp_game\out\build_perf
.PARAMETER Target    Alvo do ninja (padrao ps2EntryRunner)

.EXAMPLE
  cd ghost_pc; powershell -NoProfile -File ..\tools\build.ps1
  powershell -NoProfile -File tools\build.ps1 -GameDir ghost_pc -BuildDir work\ps2recomp_game\out\build_dev
  powershell -NoProfile -File tools\build.ps1 -GameDir ghost_pc -DryRun      # ninja -n
#>
param(
    [string]$GameDir = (Get-Location).Path,
    [string]$BuildDir = 'work\ps2recomp_game\out\build_perf',
    [string]$Target = 'ps2EntryRunner',
    [int]$Jobs = 14,                       # nao passe de 14 (16 threads fisicas, outros agentes)
    [int]$LockTimeoutSec = 1800,
    [string]$Vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat',
    [string]$Ninja = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe',
    [switch]$DryRun,
    [switch]$Explain,
    [switch]$Recompact,
    [switch]$NoLock
)
$ErrorActionPreference = 'Stop'
$GameDir = (Resolve-Path $GameDir).Path
if (-not [System.IO.Path]::IsPathRooted($BuildDir)) { $BuildDir = Join-Path $GameDir $BuildDir }
$logDir = Join-Path $GameDir 'logs'
New-Item -ItemType Directory -Force $logDir | Out-Null
$log = Join-Path $logDir ('build_{0}.log' -f (Get-Date -Format 'yyyyMMdd_HHmmss'))
function Log([string]$lvl, [string]$msg) {
    $line = '{0} [{1}] {2}' -f (Get-Date -Format 's'), $lvl, $msg
    Write-Host $line
    Add-Content -Encoding utf8 -Path $log -Value $line
}
foreach ($p in @($Vcvars, $Ninja, (Join-Path $BuildDir 'build.ninja'))) {
    if (-not (Test-Path $p)) { Log 'ERROR' "nao encontrado: $p"; exit 2 }
}
$stateFile = Join-Path $BuildDir '.build_ps1_state'
$ninjaLog = Join-Path $BuildDir '.ninja_log'
$ninjaDeps = Join-Path $BuildDir '.ninja_deps'

function Get-DepState {
    $n = 0; $d = 0
    if (Test-Path $ninjaLog) { $n = (Get-Content $ninjaLog | Measure-Object -Line).Lines }
    if (Test-Path $ninjaDeps) { $d = (Get-Item $ninjaDeps).Length }
    [pscustomobject]@{ LogLines = $n; DepsBytes = $d }
}
function Save-DepState { $s = Get-DepState; "$($s.LogLines) $($s.DepsBytes)" | Set-Content -Encoding ascii $stateFile }

# ---- lock ----
$lockDir = Join-Path $GameDir 'work\BUILD.lock.d'
$haveLock = $false
$sw = [System.Diagnostics.Stopwatch]::StartNew()
if (-not $NoLock -and -not $DryRun) {
    $waited = $false
    while ($true) {
        try { New-Item -ItemType Directory -Path $lockDir -ErrorAction Stop | Out-Null; $haveLock = $true; break } catch { }
        if (-not $waited) { Log 'WARN' "lock ocupado ($lockDir); aguardando ate ${LockTimeoutSec}s"; $waited = $true }
        if ($sw.Elapsed.TotalSeconds -gt $LockTimeoutSec) { Log 'ERROR' 'timeout esperando o lock'; exit 3 }
        Start-Sleep -Seconds 5
    }
    if ($waited) { Log 'INFO' ('lock obtido apos {0:N0}s' -f $sw.Elapsed.TotalSeconds) }
}
$code = 1
try {
    # ---- rastreio de dependencias ----
    $cur = Get-DepState
    if (Test-Path $stateFile) {
        $prev = (Get-Content $stateFile -TotalCount 1).Split(' ')
        $pl = [int]$prev[0]; $pd = [long]$prev[1]
        if ($cur.LogLines -lt $pl * 0.5) { Log 'WARN' ".ninja_log encolheu ($pl -> $($cur.LogLines) linhas): foi descartado/reescrito. Unidades sem entrada refazem (seguro, mas lento); confira se outro ninja (WinGet?) tocou este build." }
        if ($pd -gt 0 -and $cur.DepsBytes -lt $pd * 0.5) { Log 'WARN' ".ninja_deps encolheu ($pd -> $($cur.DepsBytes) bytes): dependencias de header perdidas; unidades sem deps refazem." }
    } else { Log 'INFO' 'sem estado anterior (.build_ps1_state); sera criado ao fim' }
    if (-not (Test-Path $ninjaDeps)) { Log 'WARN' '.ninja_deps ausente: o proximo build refaz tudo que usa deps (esperado so no 1o build).' }

    # ---- build ----
    $flags = "-j$Jobs"
    if ($DryRun) { $flags += ' -n' }
    if ($Explain) { $flags += ' -d explain' }
    Log 'INFO' "build: dir=$BuildDir alvo=$Target ninja=$flags log=$log"
    $cmd = 'call "{0}" >nul && cd /d "{1}" && "{2}" {3} {4} 2>&1' -f $Vcvars, $BuildDir, $Ninja, $flags, $Target
    $bt = [System.Diagnostics.Stopwatch]::StartNew()
    $starting = $false
    & cmd.exe /c $cmd | ForEach-Object {
        $l = [string]$_
        if ($l -match 'starting over|build log version|premature end') { $starting = $true }
        Add-Content -Encoding utf8 -Path $log -Value $l
        # progresso compacto no console: so linhas de status, erro e aviso
        if ($l -match '^\[\d+/\d+\]' -or $l -match 'error|warning:|FAILED|ninja:') { Write-Host $l }
    }
    $code = $LASTEXITCODE
    $bt.Stop()
    if ($starting) { Log 'WARN' 'ninja avisou que reiniciou o log/deps ("starting over"): o rastreio foi perdido nesta rodada.' }
    if ($code -eq 0 -and -not $DryRun) {
        if ($Recompact) {
            & cmd.exe /c ('call "{0}" >nul && cd /d "{1}" && "{2}" -t recompact' -f $Vcvars, $BuildDir, $Ninja) | Out-Null
            Log 'INFO' 'ninja -t recompact feito'
        }
        Save-DepState
    }
    Log $(if ($code -eq 0) { 'INFO' } else { 'ERROR' }) ('build terminou: codigo={0} tempo={1:N1}s (total com espera {2:N1}s)' -f $code, $bt.Elapsed.TotalSeconds, $sw.Elapsed.TotalSeconds)
}
finally {
    if ($haveLock) { Remove-Item -Recurse -Force $lockDir -ErrorAction SilentlyContinue; Log 'INFO' 'lock liberado' }
}
exit $code
