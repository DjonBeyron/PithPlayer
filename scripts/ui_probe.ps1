# Общие приёмы для живых проверок интерфейса: окно, снимки, мышь,
# клавиатура, разбор лога и данных песочницы.
#
# Подключается точкой: . "$PSScriptRoot\ui_probe.ps1"

Add-Type -AssemblyName System.Drawing

$probeSource = @"
using System;
using System.Runtime.InteropServices;
public class PithProbe {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, int e);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, int e);
    [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint type);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);

    // Нажатие и отпускание по отдельности — для перетаскивания.
    public static void Press() { mouse_event(0x0002, 0, 0, 0, 0); }
    public static void Release() { mouse_event(0x0004, 0, 0, 0, 0); }

    public static void Click() {
        mouse_event(0x0002, 0, 0, 0, 0);
        System.Threading.Thread.Sleep(60);
        mouse_event(0x0004, 0, 0, 0, 0);
    }

    // Скан-код обязателен: winit смотрит на физическое положение клавиши,
    // а не на символ. Без него до плеера не доходит ни одна клавиша.
    //
    // Стрелкам и соседям по блоку нужен ещё и признак расширенной клавиши:
    // без него тот же скан-код означает цифровую клавиатуру, и плеер
    // получает Numpad6 вместо «вправо».
    public static bool IsExtended(byte vk) {
        return vk == 0x25 || vk == 0x26 || vk == 0x27 || vk == 0x28 ||
               vk == 0x21 || vk == 0x22 || vk == 0x23 || vk == 0x24 ||
               vk == 0x2D || vk == 0x2E;
    }

    public static void Key(byte vk, bool ctrl) {
        byte scan = (byte)MapVirtualKey(vk, 0);
        uint extended = IsExtended(vk) ? 0x0001u : 0u;

        if (ctrl) {
            byte ctrlScan = (byte)MapVirtualKey(0x11, 0);
            keybd_event(0x11, ctrlScan, 0, 0);
            System.Threading.Thread.Sleep(40);
        }
        keybd_event(vk, scan, extended, 0);
        System.Threading.Thread.Sleep(40);
        keybd_event(vk, scan, extended | 0x0002, 0);
        if (ctrl) {
            byte ctrlScan = (byte)MapVirtualKey(0x11, 0);
            System.Threading.Thread.Sleep(40);
            keybd_event(0x11, ctrlScan, 0x0002, 0);
        }
    }

    // Обычное закрытие окна — то же, что щелчок по крестику.
    public static void Close(IntPtr h) { PostMessage(h, 0x0010, IntPtr.Zero, IntPtr.Zero); }

    // ---- дочерние окна: актёры, интеграции, выгрузка ----

    public delegate bool EnumProc(IntPtr h, IntPtr p);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr p);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);

    public static System.Collections.Generic.List<IntPtr> WindowsOfProcess(int want) {
        var found = new System.Collections.Generic.List<IntPtr>();
        EnumWindows((h, p) => {
            uint pid;
            GetWindowThreadProcessId(h, out pid);
            if (pid == (uint)want && IsWindowVisible(h)) found.Add(h);
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static string TitleOf(IntPtr h) {
        var sb = new System.Text.StringBuilder(400);
        GetWindowText(h, sb, 400);
        return sb.ToString();
    }
}
"@
if (-not ("PithProbe" -as [type])) { Add-Type -TypeDefinition $probeSource }

# ------------------------------------------------------------------ окно ---

# Окно, по которому проверке разрешено щёлкать.
#
# Пока проверка идёт, пользователь может переключиться на своё окно —
# и тогда нажатие уйдёт туда. Помнить цель и сверяться с ней перед каждым
# нажатием дешевле, чем разбираться, что мы натворили в чужой программе.
$script:PithTarget = [IntPtr]::Zero

function Set-PithForeground($proc) {
    $script:PithTarget = $proc.MainWindowHandle

    for ($i = 0; $i -lt 10; $i++) {
        [PithProbe]::ShowWindow($proc.MainWindowHandle, 5) | Out-Null
        [PithProbe]::BringWindowToTop($proc.MainWindowHandle) | Out-Null
        [PithProbe]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
        Start-Sleep -Milliseconds 400
        if ([PithProbe]::GetForegroundWindow() -eq $proc.MainWindowHandle) { return $true }
    }
    return $false
}

function Get-PithRect($proc) {
    $rect = New-Object PithProbe+RECT
    [PithProbe]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null
    return $rect
}

# Область дочернего окна плеера по его заголовку.
#
# Окна актёров, интеграций и выгрузки — отдельные окна системы, и
# MainWindowHandle указывает не на них. $null, если такого окна нет.
function Get-PithChildRect($proc, [string]$title) {
    foreach ($h in [PithProbe]::WindowsOfProcess($proc.Id)) {
        if ([PithProbe]::TitleOf($h) -eq $title) {
            $rect = New-Object PithProbe+RECT
            [PithProbe]::GetWindowRect($h, [ref]$rect) | Out-Null
            return $rect
        }
    }
    return $null
}

# Закрывает окно как пользователь и ждёт завершения процесса.
function Close-PithWindow($proc, [int]$timeoutSec) {
    [PithProbe]::Close($proc.MainWindowHandle)
    return $proc.WaitForExit($timeoutSec * 1000)
}

# ---------------------------------------------------------------- снимки ---

function Get-PithShot($rect) {
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    $bmp = New-Object System.Drawing.Bitmap $width, $height
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
    $gfx.Dispose()
    return $bmp
}

function Save-PithShot($bmp, [string]$path) {
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
}

# Область кадра видео: середина окна без панели замеров и нижней панели.
function Get-PithVideoRegion($rect) {
    $w = $rect.Right - $rect.Left
    $h = $rect.Bottom - $rect.Top
    return @([int]($w * 0.05), [int]($h * 0.30), [int]($w * 0.70), [int]($h * 0.75))
}

# Полоса над линией перемотки — там появляется окошко предпросмотра.
function Get-PithPreviewRegion($rect) {
    $w = $rect.Right - $rect.Left
    $h = $rect.Bottom - $rect.Top
    # Каждое слагаемое в скобках: запятая в PowerShell связывает крепче
    # минуса, и без них вычитание досталось бы уже собранному массиву.
    return @([int]($w * 0.25), ($h - 190), [int]($w * 0.85), ($h - 60))
}

# Одинаковы ли снимки в этой области. Точки берутся через одну: разницу
# в кадре видно и по редкой сетке, а полный обход занимал бы секунды.
function Test-PithSame($a, $b, $region, [int]$step = 6, [int]$tolerance = 12) {
    $x0, $y0, $x1, $y1 = $region
    $x1 = [Math]::Min($x1, [Math]::Min($a.Width, $b.Width) - 1)
    $y1 = [Math]::Min($y1, [Math]::Min($a.Height, $b.Height) - 1)

    $different = 0
    for ($y = $y0; $y -lt $y1; $y += $step) {
        for ($x = $x0; $x -lt $x1; $x += $step) {
            $p = $a.GetPixel($x, $y)
            $q = $b.GetPixel($x, $y)
            if ([Math]::Abs($p.R - $q.R) -gt $tolerance -or
                [Math]::Abs($p.G - $q.G) -gt $tolerance -or
                [Math]::Abs($p.B - $q.B) -gt $tolerance) {
                $different++
                # Десяток разошедшихся точек — уже не дрожание сжатия.
                if ($different -gt 10) { return $false }
            }
        }
    }
    return $true
}

# ------------------------------------------------------- мышь и клавиши ---

function Set-PithCursor([int]$x, [int]$y) {
    [PithProbe]::SetCursorPos($x, $y) | Out-Null
}

# Плавный подвод: одиночная установка позиции не порождает событий движения,
# и окно продолжает считать курсор на прежнем месте.
function Move-PithCursorTo([int]$x, [int]$y) {
    Add-Type -AssemblyName System.Windows.Forms
    $from = [System.Windows.Forms.Cursor]::Position
    $steps = 12
    for ($i = 1; $i -le $steps; $i++) {
        $ix = [int]($from.X + ($x - $from.X) * $i / $steps)
        $iy = [int]($from.Y + ($y - $from.Y) * $i / $steps)
        [PithProbe]::SetCursorPos($ix, $iy) | Out-Null
        Start-Sleep -Milliseconds 40
    }
}

function Click-PithAt([int]$x, [int]$y) {
    Move-PithCursorTo $x $y
    Start-Sleep -Milliseconds 250

    # Нажимаем, только если впереди по-прежнему плеер.
    if ($script:PithTarget -ne [IntPtr]::Zero -and
        [PithProbe]::GetForegroundWindow() -ne $script:PithTarget) {
        throw "проверка остановлена: впереди не плеер, нажатие ушло бы в чужое окно"
    }

    [PithProbe]::Click()
    Start-Sleep -Milliseconds 300
}

$pithKeys = @{
    'space' = 0x20; 'right' = 0x27; 'left' = 0x25; 'escape' = 0x1B; 'f' = 0x46; 'v' = 0x56; 't' = 0x54; 'enter' = 0x0D
    # 'a' — окно актёров.
    'a' = 0x41
}

function Send-PithKey([string]$name, [switch]$Ctrl) {
    $vk = $pithKeys[$name]
    if (-not $vk) { throw "неизвестная клавиша: $name" }
    [PithProbe]::Key([byte]$vk, [bool]$Ctrl)
    Start-Sleep -Milliseconds 200
}

# -------------------------------------------------------------------- лог ---

function Get-PithLogText([string]$path) {
    if (-not (Test-Path $path)) { return "" }
    # Лог открыт плеером на запись — читаем копию, иначе доступ запрещён.
    $copy = [System.IO.Path]::GetTempFileName()
    Copy-Item $path $copy -Force
    $text = Get-Content $copy -Raw -Encoding UTF8
    Remove-Item $copy -Force -ErrorAction SilentlyContinue
    if ($null -eq $text) { return "" }
    return $text
}

function Test-PithLog([string]$path, [string]$pattern) {
    return (Get-PithLogText $path) -match $pattern
}

# Строки уровня ERROR и WARN: в готовом к выпуску плеере их быть не должно.
function Get-PithLogErrors([string]$path) {
    $text = Get-PithLogText $path
    if ($text -eq "") { return @() }

    $lines = $text -split "`r?`n" | Where-Object { $_ -match "\s(ERROR|WARN)\s" }

    # Не наши ошибки: необязательные шрифты и жалоба egui на отсутствие
    # запасного знака в шрифте значков — на вид плеера они не влияют.
    # Заминки кадра разбираются отдельной проверкой, у них своя мера.
    $allowed = "шрифт значков не найден|Comfortaa не найдена|replacement characters|задержала кадр"
    return @($lines | Where-Object { $_ -notmatch $allowed })
}

# Самая долгая заминка кадра интерфейса, миллисекунды.
#
# Плеер сам отмечает в логе операции, задержавшие кадр. Короткие заминки
# на открытии файла — обычное дело: mpv в это время занят, и обращения
# к его свойствам ждут. Заметная глазу остановка — уже другое дело.
function Get-PithWorstStall([string]$path) {
    $text = Get-PithLogText $path
    if ($text -eq "") { return 0 }

    $values = [regex]::Matches($text, "задержала кадр интерфейса.*?ms=(\d+)") |
        ForEach-Object { [int]$_.Groups[1].Value }

    if (-not $values) { return 0 }
    return ($values | Measure-Object -Maximum).Maximum
}

# ------------------------------------------------------- данные песочницы ---

function Get-PithBookmarkCount([string]$path) {
    if (-not (Test-Path $path)) { return 0 }
    $data = Get-Content $path -Raw -Encoding UTF8 | ConvertFrom-Json
    $sum = 0
    foreach ($video in $data.videos) {
        foreach ($list in $video.lists) { $sum += @($list.bookmarks).Count }
    }
    return $sum
}

# Имя первой подписанной закладки — по нему видно, что «+» в поиске
# подставил текст реплики.
function Get-PithNamedBookmark([string]$path) {
    if (-not (Test-Path $path)) { return $null }
    $data = Get-Content $path -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($video in $data.videos) {
        foreach ($list in $video.lists) {
            foreach ($bookmark in @($list.bookmarks)) {
                if ($bookmark.name) { return $bookmark.name }
            }
        }
    }
    return $null
}

# ---------------------------------------------------------------- нарезка ---

# Нажимает в панели отрезков кнопку «Вырезать отрезки».
#
# Кнопка ищется по цвету: она единственная жёлтая в тёмном ящике. Считать
# её положение по числу закладок нельзя — список меняет высоту.
function Start-PithExtraction($rect, [string]$outputDir) {
    $shot = Get-PithShot $rect
    $w = $shot.Width

    # Ящик шириной 320 прижат к правому краю окна. Берём с запасом:
    # рамка окна сдвигает его на несколько точек.
    $left = $w - 350
    $right = $w - 10

    # Содержимое ящика идёт сверху вниз: переключатель списка, закладки,
    # кнопки нарезки. Берём самую нижнюю широкую светлую полосу — выше неё
    # такой же на вид бывает только переключатель списка.
    $rows = @()
    $start = $null

    for ($y = 120; $y -lt ($shot.Height - 60); $y += 2) {
        $filled = 0
        for ($x = $left; $x -lt $right; $x += 3) {
            $c = $shot.GetPixel($x, $y)
            # «Вырезать отрезки» — единственная жёлтая кнопка панели.
            # Считаем все её точки, а не подряд идущие: надпись внутри
            # рвёт заливку на куски.
            if ($c.R -gt 200 -and $c.G -gt 150 -and $c.B -lt 100) {
                $filled++
            }
        }

        # Полсотни точек заливки — это кнопка, а не тонкая черта.
        $wide = $filled -gt 16

        if ($wide -and -not $start) { $start = $y }
        if (-not $wide -and $start) {
            $rows += , @($start, ($y - 2))
            $start = $null
        }
    }

    $shot.Dispose()
    if ($rows.Count -eq 0) { return $null }

    $last = $rows[$rows.Count - 1]
    $middle = [int](($last[0] + $last[1]) / 2)

    # «Вырезать отрезки» — левая кнопка в строке, считаем от края ящика.
    Click-PithAt ($rect.Left + $w - 300) ($rect.Top + $middle)

    # Нарезка идёт внешним процессом: ждём появления файла.
    for ($i = 0; $i -lt 30; $i++) {
        $made = Get-ChildItem $outputDir -File -ErrorAction SilentlyContinue |
            Where-Object { $_.Extension -in ".mp4", ".mkv", ".mov" }
        if ($made) { return $made[0].FullName }
        Start-Sleep -Milliseconds 500
    }

    return $null
}
