# Открывает контекстное меню правым щелчком и снимает окно.
#
# Запуск: powershell -File scripts\menu_test.ps1 -File "видео.mkv"

param(
    [Parameter(Mandatory = $true)][string]$File,
    [string]$Out = "shot_menu.png",
    [int]$Wait = 8,
    [string]$Exe = "target\release\pith-player.exe"
)

Add-Type -AssemblyName System.Drawing

$source = @"
using System;
using System.Runtime.InteropServices;
public class MenuShot {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, int e);
    public const uint RIGHTDOWN = 0x0008;
    public const uint RIGHTUP = 0x0010;
}
"@
if (-not ("MenuShot" -as [type])) { Add-Type -TypeDefinition $source }

$proc = Start-Process -FilePath $Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

for ($i = 0; $i -lt 8; $i++) {
    [MenuShot]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 400
    if ([MenuShot]::GetForegroundWindow() -eq $proc.MainWindowHandle) { break }
}

$rect = New-Object MenuShot+RECT
[MenuShot]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null

$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top

# Щёлкаем в левой части кадра, чтобы меню раскрылось вправо и целиком.
$x = [int]($rect.Left + $width * 0.25)
$y = [int]($rect.Top + $height * 0.35)

Write-Host "правый щелчок в ($x, $y)"
[MenuShot]::SetCursorPos($x, $y)
Start-Sleep -Milliseconds 300
[MenuShot]::mouse_event([MenuShot]::RIGHTDOWN, 0, 0, 0, 0)
Start-Sleep -Milliseconds 60
[MenuShot]::mouse_event([MenuShot]::RIGHTUP, 0, 0, 0, 0)
Start-Sleep -Milliseconds 700

# Наводим курсор на пункт с подменю: оно обязано раскрыться само,
# без щелчка, и лечь сбоку от меню.
$itemX = $x + 60
$itemY = $y + 52
Write-Host "навожу на «Аудиодорожка» ($itemX, $itemY)"
[MenuShot]::SetCursorPos($itemX, $itemY)
Start-Sleep -Milliseconds 1200

$bmp = New-Object System.Drawing.Bitmap $width, $height
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
$bmp.Save((Join-Path $PWD $Out), [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()

Write-Host "снимок сохранён: $Out"
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
