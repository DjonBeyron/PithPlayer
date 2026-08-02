# Проверяет, что щелчок по найденной реплике перематывает воспроизведение.
#
# Запуск: powershell -File scripts\search_click_test.ps1 -File "видео.mkv"

param(
    [Parameter(Mandatory = $true)][string]$File,
    [string]$Query = "know",
    [int]$Wait = 12,
    [string]$Exe = "target\release\pith-player.exe"
)

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

$source = @"
using System;
using System.Runtime.InteropServices;
public class ClickShot {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, int e);
    public const uint LEFTDOWN = 0x0002;
    public const uint LEFTUP = 0x0004;
}
"@
if (-not ("ClickShot" -as [type])) { Add-Type -TypeDefinition $source }

$proc = Start-Process -FilePath $Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

for ($i = 0; $i -lt 10; $i++) {
    [ClickShot]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 400
    if ([ClickShot]::GetForegroundWindow() -eq $proc.MainWindowHandle) { break }
}

[System.Windows.Forms.SendKeys]::SendWait($Query)
Start-Sleep -Seconds 2

$rect = New-Object ClickShot+RECT
[ClickShot]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null

# Окно поиска стоит в левом верхнем углу; первая строка результатов —
# примерно на 143 точках ниже его края.
$x = $rect.Left + 200
$y = $rect.Top + 143

Write-Host "навожу на первую найденную реплику ($x, $y)"
[ClickShot]::SetCursorPos($x, $y)
Start-Sleep -Milliseconds 700

# Снимок с наведением: видно подсветку строки.
$bmp = New-Object System.Drawing.Bitmap ($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top)
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
$bmp.Save((Join-Path $PWD "shot_search_hover.png"), [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()

Write-Host "щёлкаю"
[ClickShot]::mouse_event([ClickShot]::LEFTDOWN, 0, 0, 0, 0)
Start-Sleep -Milliseconds 60
[ClickShot]::mouse_event([ClickShot]::LEFTUP, 0, 0, 0, 0)
Start-Sleep -Seconds 2

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Write-Host "готово"
