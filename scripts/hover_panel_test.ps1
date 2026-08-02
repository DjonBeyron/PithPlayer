# Проверяет, что панель отрезков выезжает при наведении на правый край
# и прячется, когда курсор уходит влево.
#
# Запуск: powershell -File scripts\hover_panel_test.ps1 -File "видео.mkv"

param(
    [Parameter(Mandatory = $true)][string]$File,
    [int]$Wait = 8,
    [string]$Exe = "target\release\pith-player.exe"
)

Add-Type -AssemblyName System.Drawing

$source = @"
using System;
using System.Runtime.InteropServices;
public class HoverTest {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
}
"@
if (-not ("HoverTest" -as [type])) { Add-Type -TypeDefinition $source }

function Save-Shot($rect, $name) {
    $bmp = New-Object System.Drawing.Bitmap ($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
    $bmp.Save((Join-Path $PWD $name), [System.Drawing.Imaging.ImageFormat]::Png)
    $gfx.Dispose()
    $bmp.Dispose()
    Write-Host "снимок: $name"
}

$proc = Start-Process -FilePath $Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

for ($i = 0; $i -lt 10; $i++) {
    [HoverTest]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 400
    if ([HoverTest]::GetForegroundWindow() -eq $proc.MainWindowHandle) { break }
}

$rect = New-Object HoverTest+RECT
[HoverTest]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null

$midY = [int]($rect.Top + ($rect.Bottom - $rect.Top) / 2)

Write-Host "курсор в центре — панели быть не должно"
[HoverTest]::SetCursorPos([int](($rect.Left + $rect.Right) / 2), $midY)
Start-Sleep -Seconds 1
Save-Shot $rect "shot_panel_hidden.png"

Write-Host "курсор к правому краю — панель обязана выехать"

# Двигаем плавно: одиночная установка позиции не порождает событий,
# и окно продолжает считать курсор на прежнем месте.
for ($x = ($rect.Right - 300); $x -lt ($rect.Right - 15); $x += 20) {
    [HoverTest]::SetCursorPos($x, $midY)
    Start-Sleep -Milliseconds 40
}
Start-Sleep -Seconds 1
Save-Shot $rect "shot_panel_shown.png"

Write-Host "курсор обратно в центр — панель обязана спрятаться"
$center = [int](($rect.Left + $rect.Right) / 2)
for ($x = ($rect.Right - 15); $x -gt $center; $x -= 20) {
    [HoverTest]::SetCursorPos($x, $midY)
    Start-Sleep -Milliseconds 40
}
Start-Sleep -Seconds 1
Save-Shot $rect "shot_panel_hidden_again.png"

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
