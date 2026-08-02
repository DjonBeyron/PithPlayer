# Проверяет именованные списки отрезков (PLAN.md §6.5, этап 5):
# панель показывает переключатель и закладки, контекстное меню открывает
# подменю списков, диалог создаёт новый список и делает его активным.
#
# Запуск: powershell -File scripts\lists_test.ps1 -File "видео.mkv"
#
# Снимок берётся через `PrintWindow`, а не с экрана: копия экрана отстаёт
# от окна на секунду-другую и показывает уже неверное состояние.
#
# Работает в песочнице (см. sandbox.ps1): закладки живого пользователя
# не затрагиваются.

param(
    [Parameter(Mandatory = $true)][string]$File,
    [int]$Wait = 6,
    [string]$Release = "target\release"
)

. "$PSScriptRoot\sandbox.ps1"

Add-Type -AssemblyName System.Drawing

$source = @"
using System;
using System.Drawing;
using System.Runtime.InteropServices;
public class ListsTest {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, int e);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, int e);
    [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint type);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);

    // Пауза между нажатием и отпусканием: мгновенный щелчок egui иногда
    // не засчитывает.
    public static void Click() {
        mouse_event(0x0002,0,0,0,0);
        System.Threading.Thread.Sleep(60);
        mouse_event(0x0004,0,0,0,0);
    }
    public static void RightClick() { mouse_event(0x0008,0,0,0,0); mouse_event(0x0010,0,0,0,0); }

    // Скан-код обязателен: winit берёт физическое положение клавиши,
    // а с нулевым скан-кодом оно не определяется.
    public static void Key(byte vk) {
        byte scan = (byte)MapVirtualKey(vk, 0);
        keybd_event(vk, scan, 0, 0);
        System.Threading.Thread.Sleep(40);
        keybd_event(vk, scan, 0x0002, 0);
    }

    // PW_RENDERFULLCONTENT = 2: без него окна с аппаратной отрисовкой
    // отдают пустой прямоугольник.
    public static void Shot(IntPtr h, string path) {
        RECT r;
        GetWindowRect(h, out r);
        using (var bmp = new Bitmap(r.Right - r.Left, r.Bottom - r.Top))
        using (var gfx = Graphics.FromImage(bmp)) {
            IntPtr hdc = gfx.GetHdc();
            PrintWindow(h, hdc, 2);
            gfx.ReleaseHdc(hdc);
            bmp.Save(path, System.Drawing.Imaging.ImageFormat.Png);
        }
    }
}
"@
if (-not ("ListsTest" -as [type])) {
    Add-Type -TypeDefinition $source -ReferencedAssemblies System.Drawing
}

function Save-Shot($proc, $name) {
    [ListsTest]::Shot($proc.MainWindowHandle, (Join-Path $PWD $name))
    Write-Host "снимок: $name"
}

# Плавное движение: одиночная установка позиции не порождает событий,
# и окно продолжает считать курсор на прежнем месте.
function Move-To($x, $y) {
    [ListsTest]::SetCursorPos($x - 40, $y); Start-Sleep -Milliseconds 100
    [ListsTest]::SetCursorPos($x - 10, $y); Start-Sleep -Milliseconds 100
    [ListsTest]::SetCursorPos($x, $y); Start-Sleep -Milliseconds 300
}

$box = New-PithSandbox -Release $Release

$proc = Start-Process -FilePath $box.Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

for ($i = 0; $i -lt 10; $i++) {
    [ListsTest]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 400
}

$rect = New-Object ListsTest+RECT
[ListsTest]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null

$cx = [int](($rect.Left + $rect.Right) / 2)
$cy = [int](($rect.Top + $rect.Bottom) / 2)
$midY = $cy

# Щелчок отдаёт фокус клавиатуры: иначе клавиши уходят системным
# всплывающим окнам, а не плееру.
Move-To $cx $cy
[ListsTest]::Click()
Start-Sleep -Milliseconds 400

Write-Host "ставлю закладку клавишей T"
[ListsTest]::Key(0x54)
Start-Sleep -Seconds 1

Write-Host "вывожу панель отрезков"
for ($x = ($rect.Right - 300); $x -lt ($rect.Right - 15); $x += 15) {
    [ListsTest]::SetCursorPos($x, $midY)
    Start-Sleep -Milliseconds 60
}
Start-Sleep -Seconds 3
Save-Shot $proc "shot_lists_panel.png"

Write-Host "открываю подменю списков в контекстном меню"
Move-To $cx $cy
[ListsTest]::RightClick()
Start-Sleep -Seconds 2
Move-To ($cx + 95) ($cy + 76)
Start-Sleep -Seconds 2
Save-Shot $proc "shot_lists_menu.png"

Write-Host "выбираю «Новый список…»"
[ListsTest]::SetCursorPos(($cx + 230), ($cy + 90)); Start-Sleep -Milliseconds 200
[ListsTest]::SetCursorPos(($cx + 294), ($cy + 102)); Start-Sleep -Milliseconds 300
[ListsTest]::SetCursorPos(($cx + 294), ($cy + 110)); Start-Sleep -Milliseconds 400
[ListsTest]::Click()
Start-Sleep -Seconds 2

Write-Host "ввожу имя списка"
foreach ($vk in @(0x44, 0x49, 0x41, 0x4C, 0x4F, 0x47)) {
    [ListsTest]::Key([byte]$vk); Start-Sleep -Milliseconds 90
}
Start-Sleep -Seconds 1
Save-Shot $proc "shot_lists_dialog.png"

Write-Host "сохраняю по Enter"
[ListsTest]::Key(0x0D)
Start-Sleep -Seconds 2

for ($x = ($rect.Right - 300); $x -lt ($rect.Right - 15); $x += 15) {
    [ListsTest]::SetCursorPos($x, $midY)
    Start-Sleep -Milliseconds 60
}
Start-Sleep -Seconds 3
Save-Shot $proc "shot_lists_created.png"

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue

$data = Get-PithSandboxBookmarks $box
if ($data) {
    foreach ($video in $data.videos) {
        $names = ($video.lists | ForEach-Object { "$($_.name) ($($_.bookmarks.Count))" }) -join ", "
        Write-Host "$($video.video_file_name): активен «$($video.active_list)»; списки: $names"
    }
} else {
    Write-Host "файл закладок не создан"
}

Remove-PithSandbox $box
