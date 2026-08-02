# Проверка нарезки: ставит закладки клавишей T и запускает вырезание.
#
# Запуск: powershell -File scripts\extract_test.ps1 -File "видео.mkv"
#
# Работает в песочнице (см. sandbox.ps1): настройки, закладки и позиции
# просмотра живого пользователя не затрагиваются.

param(
    [Parameter(Mandatory = $true)][string]$File,
    [int]$Wait = 8,
    [string]$Release = "target\release"
)

. "$PSScriptRoot\sandbox.ps1"

$source = @"
using System;
using System.Runtime.InteropServices;
public class ExtractTest {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, int e);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, int e);
    [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint type);
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    public static void Click() {
        mouse_event(0x0002,0,0,0,0);
        System.Threading.Thread.Sleep(60);
        mouse_event(0x0004,0,0,0,0);
    }
    // Скан-код обязателен: winit смотрит на физическое положение клавиши.
    public static void Key(byte vk) {
        byte scan = (byte)MapVirtualKey(vk, 0);
        keybd_event(vk, scan, 0, 0);
        System.Threading.Thread.Sleep(40);
        keybd_event(vk, scan, 0x0002, 0);
    }
}
"@
if (-not ("ExtractTest" -as [type])) { Add-Type -TypeDefinition $source }

$outDir = Join-Path $env:TEMP "pith_fragments_test"
Remove-Item $outDir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

# Короткие отрезки и своя папка вывода — прямо в настройках песочницы.
$box = New-PithSandbox -Release $Release -Settings @{
    version   = 1
    fragments = @{
        output_dir    = $outDir
        duration_sec  = 5
        buffer_sec    = 2
        reencode      = $false
        parallel_jobs = 0
    }
}

$proc = Start-Process -FilePath $box.Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

[ExtractTest]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 600

# Щелчок по видео забирает фокус клавиатуры у системных всплывающих окон.
$rect = New-Object ExtractTest+RECT
[ExtractTest]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null
[ExtractTest]::SetCursorPos([int](($rect.Left + $rect.Right) / 2), [int](($rect.Top + $rect.Bottom) / 2))
Start-Sleep -Milliseconds 300
[ExtractTest]::Click()
Start-Sleep -Milliseconds 400

Write-Host "ставлю три закладки"
foreach ($n in 1..3) {
    [ExtractTest]::Key(0x54)
    Start-Sleep -Milliseconds 1500
}

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 800

$data = Get-PithSandboxBookmarks $box
if ($data) {
    $count = ($data.videos | ForEach-Object { $_.lists[0].bookmarks.Count } | Measure-Object -Sum).Sum
    Write-Host "закладок сохранено: $count"
} else {
    Write-Host "закладки не сохранились"
}

Remove-PithSandbox $box
Write-Host "папка вывода: $outDir"
