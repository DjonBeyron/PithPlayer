# Проверка перетаскивания субтитров: тащим слой мышью и смотрим,
# изменилось ли сохранённое положение.
#
# Запуск: powershell -File scripts\drag_test.ps1 -File "видео.mkv"

param(
    [Parameter(Mandatory = $true)][string]$File,
    [int]$Wait = 42,
    [string]$Exe = "target\release\pith-player.exe"
)

$source = @"
using System;
using System.Runtime.InteropServices;
public class Mouse {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, int e);
    public const uint LEFTDOWN = 0x0002;
    public const uint LEFTUP = 0x0004;
}
"@
if (-not ("Mouse" -as [type])) { Add-Type -TypeDefinition $source }

$settings = "$env:APPDATA\PithPlayer\settings.json"
Remove-Item $settings -Force -ErrorAction SilentlyContinue

$proc = Start-Process -FilePath $Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

[Mouse]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 800

# Координаты считаем от окна плеера: субтитры стоят на 0.88 его высоты.
$rect = New-Object Mouse+RECT
[Mouse]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null

$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
$startX = [int]($rect.Left + $width / 2)

# Субтитры стоят на 0.88 высоты клиентской области, а rect включает
# заголовок и рамку — точную поправку не вычислить, поэтому перебираем.
$candidates = @(0.84, 0.86, 0.88, 0.90, 0.82) | ForEach-Object {
    [int]($rect.Top + $height * $_)
}

Write-Host "окно ${width}x${height}, пробую вертикали: $($candidates -join ', ')"

# Реплики появляются не постоянно, поэтому пробуем несколько раз:
# промах по времени — обычное дело, а не признак поломки.
for ($attempt = 1; $attempt -le 10; $attempt++) {
    $startY = $candidates[($attempt - 1) % $candidates.Count]
    [Mouse]::SetCursorPos($startX, $startY)
    Start-Sleep -Milliseconds 250
    [Mouse]::mouse_event([Mouse]::LEFTDOWN, 0, 0, 0, 0)

    # Движение мелкими шагами: egui считает перетаскивание по кадрам.
    for ($i = 1; $i -le 15; $i++) {
        [Mouse]::SetCursorPos($startX, $startY - ($i * 8))
        Start-Sleep -Milliseconds 35
    }

    [Mouse]::mouse_event([Mouse]::LEFTUP, 0, 0, 0, 0)
    Start-Sleep -Milliseconds 500

    if (Test-Path $settings) {
        Write-Host "получилось с попытки $attempt"
        break
    }

    Start-Sleep -Seconds 3
}

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

if (Test-Path $settings) {
    Write-Host "`n=== положение слоёв после перетаскивания ==="
    Get-Content $settings | Select-String -Pattern "main_subtitle" -Context 0,4
} else {
    Write-Host "файл настроек не создан — перетаскивание не сработало"
}
