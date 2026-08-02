# Проверяет, что плеер остаётся живым на плохих входных данных
# (PLAN.md, чек-лист этапа 6):
#   1. несуществующий файл в аргументах;
#   2. битый файл — мусор с расширением видео;
#   3. файл удалён или стал недоступен во время просмотра.
#
# Запуск: powershell -File scripts\robustness_test.ps1 -File "видео.mkv"
#
# Третий случай на Windows часто невозможен: пока mpv держит файл открытым,
# система не даёт его удалить. Тогда пробуем переименовать папку — так ведёт
# себя отключённая флешка или пропавший сетевой диск.

param(
    [Parameter(Mandatory = $true)][string]$File,
    [string]$Exe = "target\release\pith-player.exe"
)

Add-Type -AssemblyName System.Drawing

$source = @"
using System;
using System.Runtime.InteropServices;
public class Rob {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
}
"@
if (-not ("Rob" -as [type])) { Add-Type -TypeDefinition $source }

function Save-Shot($proc, $name) {
    $rect = New-Object Rob+RECT
    if (-not [Rob]::GetWindowRect($proc.MainWindowHandle, [ref]$rect)) { return }

    $bmp = New-Object System.Drawing.Bitmap ($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
    $bmp.Save((Join-Path $PWD $name), [System.Drawing.Imaging.ImageFormat]::Png)
    $gfx.Dispose(); $bmp.Dispose()
    Write-Host "  снимок: $name"
}

function Start-Player($argument) {
    # Пауза перед запуском обязательна: предыдущий экземпляр отпускает
    # порт защиты от второго запуска не мгновенно, и новый плеер решит,
    # что он второй, отправит путь в пустоту и молча закроется.
    Start-Sleep -Seconds 3

    $proc = Start-Process -FilePath $Exe -ArgumentList "`"$argument`"" -PassThru
    Start-Sleep -Seconds 6
    [Rob]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Seconds 1
    $proc
}

function Check-Alive($proc, $case) {
    $proc.Refresh()
    if ($proc.HasExited) {
        Write-Host "  ПРОВАЛ: плеер упал, код $($proc.ExitCode)" -ForegroundColor Red
    } elseif (-not $proc.Responding) {
        Write-Host "  ПРОВАЛ: окно не отвечает" -ForegroundColor Red
    } else {
        Write-Host "  ок: плеер жив и отвечает" -ForegroundColor Green
    }
}

$temp = Join-Path $env:TEMP "pith_robustness"
New-Item -ItemType Directory -Force $temp | Out-Null

Write-Host "`n=== 1. Несуществующий файл ==="
$proc = Start-Player (Join-Path $temp "нет-такого-файла.mkv")
Check-Alive $proc "нет файла"
Save-Shot $proc "shot_robust_missing.png"
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue

Write-Host "`n=== 2. Битый файл ==="
$broken = Join-Path $temp "broken.mp4"
$bytes = New-Object byte[] 262144
(New-Object Random 42).NextBytes($bytes)
[System.IO.File]::WriteAllBytes($broken, $bytes)

$proc = Start-Player $broken
Check-Alive $proc "битый файл"
Save-Shot $proc "shot_robust_broken.png"
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue

Write-Host "`n=== 3. Файл исчезает во время просмотра ==="
$dir = Join-Path $temp "playing"
New-Item -ItemType Directory -Force $dir | Out-Null
$copy = Join-Path $dir ([System.IO.Path]::GetFileName($File))
Copy-Item -LiteralPath $File -Destination $copy -Force

$proc = Start-Player $copy
Start-Sleep -Seconds 2

$removed = $false
try {
    Remove-Item -LiteralPath $copy -Force -ErrorAction Stop
    $removed = $true
    Write-Host "  файл удалён"
} catch {
    try {
        Rename-Item -LiteralPath $dir -NewName "playing_gone" -ErrorAction Stop
        $removed = $true
        Write-Host "  папка переименована — путь стал недоступен"
    } catch {
        Write-Host "  ни удалить, ни переименовать не вышло: $($_.Exception.Message)"
    }
}

if ($removed) {
    Start-Sleep -Seconds 5
    Check-Alive $proc "файл исчез"
    Save-Shot $proc "shot_robust_vanished.png"
}
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue

Start-Sleep -Milliseconds 500
Remove-Item -Recurse -Force $temp -ErrorAction SilentlyContinue
Write-Host "`nготово"
