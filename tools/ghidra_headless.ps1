<#
.SYNOPSIS
  Roda o Ghidra headless sobre um projeto existente com scripts da raiz e do PS2Recomp.

.DESCRIPTION
  Encapsula o que aprendemos: as aspas aninhadas quebram com `cmd /c`, entao usamos Start-Process com
  -ArgumentList; scripts leem argumentos via -postScript <script> <args...>; -scriptPath aceita varios
  diretorios separados por ';'. Saida em <GameDir>\logs\<LogName>.log.

.EXAMPLE
  # cria funcoes nos alvos ausentes e reexporta o TOML do PS2Recomp
  .\tools\ghidra_headless.ps1 -GameDir ghost_pc -Program SLES_530.20 -LogName missing_cycle `
      -Script @(
        @('CreateFunctionsAt.java','ghost_pc/logs/missing.txt'),
        @('ExportPS2Functions.java','ghost_pc/work/recomp/ghost.toml','ghost_pc/work/recomp/ghost_functions.csv'))
#>
param(
    [Parameter(Mandatory)] [string] $GameDir,
    [Parameter(Mandatory)] [string] $Program,
    [Parameter(Mandatory)] [object[]] $Script,
    [string] $Project = 'work\ghidra_project',
    [string] $ProjectName = 'spike',
    [string] $LogName = 'ghidra_headless',
    [string] $GhidraDir = 'C:\Users\guilh\Downloads\ghidra_12.1.4_PUBLIC',
    [string] $JavaHome = 'C:\Program Files\Microsoft\jdk-21.0.13.21-hotspot',
    [switch] $ReadOnly
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = (Resolve-Path (Join-Path $root $GameDir)).Path
$fwd = { param($p) ($p -replace '\\', '/') }

$scriptPath = @(
    (& $fwd (Join-Path $root 'tools\ghidra')),
    (& $fwd (Join-Path $root 'third_party\PS2Recomp\ps2xRecomp\tools\ghidra'))
) -join ';'

$args = @("`"$(& $fwd (Join-Path $game $Project))`"", $ProjectName, '-process', $Program, '-noanalysis')
if ($ReadOnly) { $args += '-readOnly' }
$args += @('-scriptPath', "`"$scriptPath`"")
foreach ($entry in $Script) {
    $parts = @($entry)
    $args += @('-postScript', $parts[0])
    foreach ($a in $parts[1..($parts.Count - 1)]) {
        # caminhos relativos a raiz do repo viram absolutos; outros valores passam como estao
        $candidate = Join-Path $root $a
        $value = if ($a -match '[\\/]' -and -not [IO.Path]::IsPathRooted($a)) { & $fwd $candidate } else { $a }
        $args += "`"$value`""
    }
}

$logDir = Join-Path $game 'logs'
New-Item -ItemType Directory -Force $logDir | Out-Null
$env:JAVA_HOME = $JavaHome
$proc = Start-Process -FilePath (Join-Path $GhidraDir 'support\analyzeHeadless.bat') -ArgumentList $args `
    -Wait -PassThru -NoNewWindow `
    -RedirectStandardOutput (Join-Path $logDir "$LogName.log") `
    -RedirectStandardError (Join-Path $logDir "$LogName.err")

Select-String -Path (Join-Path $logDir "$LogName.log") -Pattern 'ERROR|CreateFunctionsAt|CreateMissing|ApplySymbols|Exported' |
    ForEach-Object { $_.Line.Substring(0, [Math]::Min(240, $_.Line.Length)) }
exit $proc.ExitCode
