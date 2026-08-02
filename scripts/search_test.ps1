# Открывает поиск по субтитрам, вводит запрос и снимает окно.
#
# Запуск: powershell -File scripts\search_test.ps1 -File "видео.mkv" -Query "the"

param(
    [Parameter(Mandatory = $true)][string]$File,
    [string]$Query = "the",
    [string]$Out = "shot_search.png",
    [int]$Wait = 10,
    [string]$Exe = "target\release\pith-player.exe"
)

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

$source = @"
using System;
using System.Runtime.InteropServices;
public class SearchShot {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
}
"@
if (-not ("SearchShot" -as [type])) { Add-Type -TypeDefinition $source }

$proc = Start-Process -FilePath $Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

for ($i = 0; $i -lt 10; $i++) {
    [SearchShot]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 400
    if ([SearchShot]::GetForegroundWindow() -eq $proc.MainWindowHandle) { break }
}

Write-Host "открываю поиск (Ctrl+F)"
[System.Windows.Forms.SendKeys]::SendWait("^f")

# Извлечение дорожки идёт в фоне и на длинном фильме занимает секунды.
Start-Sleep -Seconds 6

Write-Host "ввожу запрос: $Query"
[System.Windows.Forms.SendKeys]::SendWait($Query)
Start-Sleep -Seconds 2

$rect = New-Object SearchShot+RECT
[SearchShot]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null

$bmp = New-Object System.Drawing.Bitmap ($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top)
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
$bmp.Save((Join-Path $PWD $Out), [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()

Write-Host "снимок сохранён: $Out"
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
