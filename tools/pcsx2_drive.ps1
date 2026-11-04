<#
.SYNOPSIS
  Dirige uma instancia do PCSX2 como referencia: captura a janela e aperta teclas em tempos definidos.

.DESCRIPTION
  Cada passo e "<segundos> <acao>", em um arquivo de texto. Acoes: shot <nome> | key <VK decimal> | quit.
  VK comuns: X=88, Return=13, Up=38, Down=40, Left=37, Right=39.
  Nao abre nem altera arquivos do jogo. As capturas vao para -ShotDir (fora do git) e o log para -LogFile.

.EXAMPLE
  .\tools\pcsx2_drive.ps1 -Script ghost_pc\pad_scripts\pcsx2_ref.txt -ShotDir ghost_pc\logs\pcsx2_shots -LogFile ghost_pc\logs\pcsx2_drive.log
#>
param(
    [Parameter(Mandatory)] [string] $Script,
    [Parameter(Mandatory)] [string] $ShotDir,
    [Parameter(Mandatory)] [string] $LogFile
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class Win {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
}
'@

function Write-Log([string] $Text) {
    $line = "{0} INFO {1}" -f (Get-Date).ToUniversalTime().ToString('s'), $Text
    Add-Content -Path $LogFile -Value $line -Encoding utf8
    Write-Output $line
}

function Get-GameWindow {
    $process = Get-Process pcsx2-qt -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if (-not $process) { throw 'PCSX2 nao esta rodando' }
    return $process.MainWindowHandle
}

function Save-Shot([string] $Name) {
    $handle = Get-GameWindow
    [Win+RECT] $rect = New-Object 'Win+RECT'
    [void][Win]::GetClientRect($handle, [ref] $rect)
    $origin = New-Object 'Win+POINT'
    [void][Win]::ClientToScreen($handle, [ref] $origin)
    $bitmap = New-Object System.Drawing.Bitmap ($rect.R - $rect.L), ($rect.B - $rect.T)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($origin.X, $origin.Y, 0, 0, $bitmap.Size)
    $bitmap.Save((Join-Path $ShotDir "$Name.png"))
    $graphics.Dispose(); $bitmap.Dispose()
}

function Send-Key([int] $VirtualKey) {
    [void][Win]::SetForegroundWindow((Get-GameWindow))
    Start-Sleep -Milliseconds 100
    [Win]::keybd_event([byte]$VirtualKey, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 120
    [Win]::keybd_event([byte]$VirtualKey, 0, 2, [UIntPtr]::Zero)
}

New-Item -ItemType Directory -Force $ShotDir | Out-Null
$start = Get-Date
foreach ($row in Get-Content $Script -Encoding UTF8) {
    if ($row -match '^\s*(#|$)') { continue }
    $parts = $row -split '\s+'
    $wait = [double]$parts[0] - ((Get-Date) - $start).TotalSeconds
    if ($wait -gt 0) { Start-Sleep -Milliseconds ([int]($wait * 1000)) }
    switch ($parts[1]) {
        'shot' { Save-Shot $parts[2]; Write-Log "shot $($parts[2])" }
        'key' { Send-Key ([int]$parts[2]); Write-Log "key $($parts[2])" }
        'quit' { Write-Log 'quit'; return }
        default { throw "acao desconhecida: $($parts[1])" }
    }
}
