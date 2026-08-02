# Снимок окна плеера — для проверки интерфейса.
# Запуск: powershell -File scripts\screenshot.ps1 -File "видео.mkv" -Out shot.png

param(
    [Parameter(Mandatory = $true)][string]$File,
    [string]$Out = "screenshot.png",
    [int]$Wait = 10,
    [string]$Exe = "target\release\pith-player.exe",
    # Клавиши, отправляемые перед снимком: например "{ENTER}" для диалогов.
    [string]$Keys = ""
)

Add-Type -AssemblyName System.Drawing

$source = @"
using System;
using System.Runtime.InteropServices;
public class WinShot {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
}
"@
if (-not ("WinShot" -as [type])) { Add-Type -TypeDefinition $source }

$proc = Start-Process -FilePath $Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

# Windows не всегда отдаёт передний план по первому требованию: без проверки
# снимок захватит чужие окна, лежащие поверх плеера, и это легко принять
# за неисправность самого плеера.
$foreground = $false
for ($i = 0; $i -lt 10; $i++) {
    [WinShot]::ShowWindow($proc.MainWindowHandle, 5) | Out-Null   # SW_SHOW
    [WinShot]::BringWindowToTop($proc.MainWindowHandle) | Out-Null
    [WinShot]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 400

    if ([WinShot]::GetForegroundWindow() -eq $proc.MainWindowHandle) {
        $foreground = $true
        break
    }
}

if (-not $foreground) {
    Write-Warning "Окно плеера не удалось вывести на передний план — снимок может захватить чужие окна"
}

Start-Sleep -Milliseconds 800

if ($Keys) {
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.SendKeys]::SendWait($Keys)
    Start-Sleep -Milliseconds 600
}

$rect = New-Object WinShot+RECT
[WinShot]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null

$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top

if ($width -le 0 -or $height -le 0) {
    Write-Error "Не удалось определить размер окна"
    Stop-Process -Id $proc.Id -Force
    exit 1
}

$bmp = New-Object System.Drawing.Bitmap $width, $height
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
$bmp.Save((Join-Path $PWD $Out), [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()

Write-Host "Снимок ${width}x${height} сохранён: $Out"
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
