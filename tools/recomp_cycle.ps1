<#
.SYNOPSIS
  Um ciclo completo da recompilacao estatica: nomes -> Ghidra -> TOML -> ps2_recomp -> C++ -> build.

.DESCRIPTION
  1. Aplica CSVs de simbolos (address,name,confidence,evidence) e/ou cria funcoes nos alvos ausentes
     (PS2X_MISSING_LOG) no projeto Ghidra headless do jogo, e reexporta o TOML do PS2Recomp.
  2. Pos-processa o TOML (input e opcoes que o exportador apaga).
  3. Roda o ps2_recomp (limpa a saida antiga: funcoes renomeadas deixam arquivos orfaos).
  4. Sincroniza o C++ gerado com a arvore de build e roda o ninja (incremental).
  Cada etapa grava log em <GameDir>\logs\ e o script para na primeira falha.

  AVISO DE COORDENACAO: as etapas 3-4 usam CPU/RAM de forma intensa. Antes de rodar, confirme com as
  outras sessoes (AGENTS.md, secao de coordenacao): "ocioso" nao significa "livre".

.EXAMPLE
  .\tools\recomp_cycle.ps1 -GameDir ghost_pc -Elf SLES_530.20 `
      -Symbols ghost_pc\docs\symbols_fio.csv,ghost_pc\docs\symbols_sdk2.csv `
      -MissingLog ghost_pc\logs\missing.txt -BuildBat C:\caminho\build_inc.bat
#>
param(
    [Parameter(Mandatory)] [string] $GameDir,
    [Parameter(Mandatory)] [string] $Elf,
    [string[]] $Symbols = @(),
    [string] $MissingLog,
    [Parameter(Mandatory)] [string] $BuildBat,
    [string] $Recompiler = 'third_party\PS2Recomp\out\build\ps2xRecomp\ps2_recomp.exe',
    [string] $BuildTree = 'work\ps2recomp_game',
    [string[]] $TomlOptions = @('low_memory_mode=true', 'output_worker_threads=8'),
    [string[]] $AddStubs = @(),   # ex.: 'ret0@0x002D3AD8' (liga a funcao nesse endereco a um handler do runtime)
    [switch] $PointerScan,        # cria funcoes nos alvos de ponteiros em dados/constantes (CreateFunctionsFromPointers.java)
    [switch] $SkipBuild
)

$ErrorActionPreference = 'Stop'
# Listas passadas por -File chegam como UMA string ("a,b"); separa por virgula para aceitar os dois jeitos.
$Symbols = @($Symbols | ForEach-Object { $_ -split ',' } | Where-Object { $_ })
$AddStubs = @($AddStubs | ForEach-Object { $_ -split ',' } | Where-Object { $_ })
$TomlOptions = @($TomlOptions | ForEach-Object { $_ -split ',' } | Where-Object { $_ })
$root = Split-Path -Parent $PSScriptRoot
$game = (Resolve-Path (Join-Path $root $GameDir)).Path
$gameRel = $GameDir.TrimEnd('\', '/')
$stopwatch = [Diagnostics.Stopwatch]::StartNew()

function Step($name, [scriptblock] $body) {
    Write-Host ("[{0,5:N0}s] {1}" -f $stopwatch.Elapsed.TotalSeconds, $name)
    & $body
    if ($LASTEXITCODE -ne 0 -and $null -ne $LASTEXITCODE) { throw "falhou: $name (codigo $LASTEXITCODE)" }
}

# 1. Ghidra headless: simbolos + alvos ausentes + exportacao
$scripts = @()
foreach ($csv in $Symbols) { $scripts += , @('ApplySymbolsCsv.java', $csv, 'low') }
if ($PointerScan) { $scripts += , @('CreateFunctionsFromPointers.java') }
if ($MissingLog) { $scripts += , @('CreateFunctionsAt.java', $MissingLog) }
$tomlPath = "$gameRel\work\recomp\ghost.toml"
$csvPath = "$gameRel\work\recomp\ghost_functions.csv"
$scripts += , @('ExportPS2Functions.java', $tomlPath, $csvPath)
Step 'Ghidra headless (nomes, funcoes, exportacao)' {
    & (Join-Path $PSScriptRoot 'ghidra_headless.ps1') -GameDir $GameDir -Program $Elf -Script $scripts -LogName 'cycle_ghidra'
}

# 2. TOML
Step 'Pos-processar TOML' {
    Push-Location $game
    try {
        $argList = @('-I', (Join-Path $root 'tools\fix_recomp_toml.py'), 'work\recomp\ghost.toml', '--input', "work\$Elf")
        foreach ($o in $TomlOptions) { $argList += @('--set', $o) }
        foreach ($s in $AddStubs) { $argList += @('--add-stub', $s) }
        & python @argList
    } finally { Pop-Location }
}

# 3. Recompilar
Step 'ps2_recomp' {
    $recomp = Join-Path $root $Recompiler
    $work = Join-Path $game 'work\recomp'
    Remove-Item (Join-Path $work 'output') -Recurse -Force -ErrorAction SilentlyContinue
    $p = Start-Process -FilePath $recomp -ArgumentList 'ghost.toml' -WorkingDirectory $work -Wait -PassThru -NoNewWindow `
        -RedirectStandardOutput (Join-Path $game 'logs\cycle_recomp.log') -RedirectStandardError (Join-Path $game 'logs\cycle_recomp.err')
    $global:LASTEXITCODE = $p.ExitCode
}

# 4. Sincronizar e compilar
Step 'Sincronizar C++ gerado' {
    Push-Location $game
    try { & python -I (Join-Path $root 'tools\sync_recomp_output.py') 'work\recomp\output' (Join-Path $BuildTree 'ps2xRuntime') } finally { Pop-Location }
}
if (-not $SkipBuild) {
    Step 'Build (ninja incremental)' {
        $p = Start-Process -FilePath cmd.exe -ArgumentList '/c', $BuildBat -Wait -PassThru -WindowStyle Hidden `
            -RedirectStandardOutput (Join-Path $game 'logs\cycle_build.log') -RedirectStandardError (Join-Path $game 'logs\cycle_build.err')
        $global:LASTEXITCODE = $p.ExitCode
    }
}
Write-Host ("[{0,5:N0}s] ciclo concluido" -f $stopwatch.Elapsed.TotalSeconds)
