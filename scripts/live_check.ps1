# Живые проверки собранного плеера: запуск, воспроизведение, перемотка,
# предпросмотр, закладки, поиск, панель отрезков, нарезка и закрытие.
#
# Всё делается в песочнице (portable.txt рядом с копией exe): данные
# пользователя в %APPDATA%\PithPlayer не трогаются.
#
# Запуск: powershell -File scripts\live_check.ps1 -File "видео.mkv" -Query "слово"
#
# Возвращает число проваленных проверок.

param(
    [Parameter(Mandatory = $true)][string]$File,
    # Слово, которое точно есть в субтитрах файла. Пусто — поиск пропускается.
    [string]$Query = "",
    [string]$OutDir = "release_check",
    # Сколько ждать открытия файла, секунды.
    [int]$Wait = 12
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
. "$PSScriptRoot\sandbox.ps1"
. "$PSScriptRoot\ui_probe.ps1"

Add-Type -AssemblyName System.Drawing

if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force $OutDir | Out-Null }
$OutDir = (Resolve-Path $OutDir).Path

$script:results = @()

function Check([string]$name, [bool]$ok, [string]$note = "") {
    $script:results += [pscustomobject]@{ Проверка = $name; Итог = $(if ($ok) { "ок" } else { "ПРОВАЛ" }); Пояснение = $note }
    $mark = $(if ($ok) { "  ок  " } else { "ПРОВАЛ" })
    Write-Host "[$mark] $name $note"
}

# ---------------------------------------------------------------- запуск ---

# Вырезанные отрезки складываем в свою временную папку: рядом с видео
# пользователя проверка ничего оставлять не должна.
$fragments = Join-Path $env:TEMP "pith_release_check_fragments"
Remove-Item $fragments -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $fragments | Out-Null

$box = New-PithSandbox -Settings @{
    version   = 1
    fragments = @{
        output_dir    = $fragments
        duration_sec  = 5
        buffer_sec    = 2
        reencode      = $false
        parallel_jobs = 0
    }
}
Write-Host "песочница: $($box.Dir)"

# Подробный лог: по нему проверяется то, чего не видно на снимке.
$env:PITH_LOG = "debug"
$proc = Start-Process -FilePath $box.Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

$focused = Set-PithForeground $proc
Check "окно выходит на передний план" $focused

# Дальше всё делается мышью и клавишами: без переднего плана нажатия
# уйдут в чужое окно, а проверки посыплются одна за другой.
if (-not $focused) {
    Write-Host ""
    Write-Host "Проверка остановлена: окно плеера не выходит вперёд."
    Write-Host "Обычно это значит, что машиной сейчас пользуются."

    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Remove-PithSandbox $box
    Remove-Item Env:\PITH_LOG -ErrorAction SilentlyContinue
    exit 1
}

$rect = Get-PithRect $proc
$log = Join-Path $box.Dir "pith-player.log"

Check "файл открылся" (Test-PithLog $log "файл загружен")

# Курсор в стороне от полосы: подсказка предпросмотра не должна попадать
# в снимки, по которым сравниваются кадры.
Set-PithCursor ([int](($rect.Left + $rect.Right) / 2)) ([int]($rect.Top + ($rect.Bottom - $rect.Top) * 0.25))
Start-Sleep -Milliseconds 500

# ------------------------------------------------------- воспроизведение ---

$a = Get-PithShot $rect
Start-Sleep -Milliseconds 700
$b = Get-PithShot $rect
Check "кадры идут" (-not (Test-PithSame $a $b (Get-PithVideoRegion $rect))) "кадр меняется сам"
Save-PithShot $b (Join-Path $OutDir "01_воспроизведение.png")

Send-PithKey "space"
Start-Sleep -Milliseconds 800
$a = Get-PithShot $rect
Start-Sleep -Milliseconds 700
$b = Get-PithShot $rect
Check "пауза по пробелу" (Test-PithSame $a $b (Get-PithVideoRegion $rect)) "кадр застыл"
Save-PithShot $b (Join-Path $OutDir "02_пауза.png")

# ------------------------------------------------------------ предпросмотр ---

Check "мозаика миниатюр собрана" (Test-PithLog $log "мозаика миниатюр (собрана|взята из кэша)")

$thumbs = Join-Path $box.Data "thumbs"
$hasThumbs = (Test-Path $thumbs) -and ((Get-ChildItem $thumbs -Filter *.jpg -ErrorAction SilentlyContinue).Count -gt 0)
Check "мозаика лежит в кэше" $hasThumbs $thumbs

$before = Get-PithShot $rect
Set-PithCursor ([int]($rect.Left + ($rect.Right - $rect.Left) * 0.6)) ($rect.Bottom - 30)
Start-Sleep -Milliseconds 900
$after = Get-PithShot $rect
$strip = Get-PithPreviewRegion $rect
Check "подсказка с кадром появляется" (-not (Test-PithSame $before $after $strip)) "над полосой что-то нарисовалось"
Save-PithShot $after (Join-Path $OutDir "03_предпросмотр.png")

# ---------------------------------------------------------------- перемотка ---

$before = Get-PithShot $rect
Click-PithAt ([int]($rect.Left + ($rect.Right - $rect.Left) * 0.5)) ($rect.Bottom - 30)
Start-Sleep -Seconds 2
$after = Get-PithShot $rect
Check "перемотка щелчком по полосе" (-not (Test-PithSame $before $after (Get-PithVideoRegion $rect))) "кадр сменился"
Save-PithShot $after (Join-Path $OutDir "04_перемотка.png")

# ----------------------------------------------------------------- закладка ---

# «+» стоит справа от полосы перемотки, перед кнопкой полного экрана.
Click-PithAt ($rect.Right - 215) ($rect.Bottom - 24)
Start-Sleep -Milliseconds 800
Save-PithShot (Get-PithShot $rect) (Join-Path $OutDir "05_закладка.png")

$bookmarks = Join-Path $box.Data "bookmarks.json"
$count = Get-PithBookmarkCount $bookmarks
Check "кнопка + ставит закладку" ($count -ge 1) "закладок: $count"

# -------------------------------------------------------------------- поиск ---

if ($Query -ne "") {
    # Запрос вставляем из буфера обмена: набор с клавиатуры зависит
    # от раскладки, а Ctrl+V — нет.
    Set-Clipboard -Value $Query
    Send-PithKey "f" -Ctrl
    Start-Sleep -Seconds 6
    Send-PithKey "v" -Ctrl
    Start-Sleep -Seconds 2
    Save-PithShot (Get-PithShot $rect) (Join-Path $OutDir "06_поиск.png")

    Check "субтитры прочитаны для поиска" (Test-PithLog $log "субтитры готовы к поиску")

    # «+» напротив первой найденной реплики.
    Click-PithAt ($rect.Left + 43) ($rect.Top + 143)
    Start-Sleep -Milliseconds 900
    Save-PithShot (Get-PithShot $rect) (Join-Path $OutDir "07_закладка_из_поиска.png")

    $named = Get-PithNamedBookmark $bookmarks
    Check "+ в поиске ставит подписанную закладку" ($named -ne $null) "имя: $named"

    Send-PithKey "escape"
    Start-Sleep -Milliseconds 500
} else {
    Check "поиск по субтитрам" $true "пропущено: не задан -Query"
}

# --------------------------------------------------------- панель отрезков ---

$midY = [int](($rect.Top + $rect.Bottom) / 2)

# Язычок показывается, когда курсор в правой трети окна.
Move-PithCursorTo ([int]($rect.Right - ($rect.Right - $rect.Left) * 0.2)) $midY
Start-Sleep -Milliseconds 800
Save-PithShot (Get-PithShot $rect) (Join-Path $OutDir "08_язычок.png")

# Панель открывается нажатием на язычок и идёт во всю высоту окна.
Click-PithAt ($rect.Right - 16) $midY
Start-Sleep -Seconds 1
Save-PithShot (Get-PithShot $rect) (Join-Path $OutDir "08_панель_отрезков.png")
Check "панель отрезков открывается язычком" (Test-PithLog $log "панель отрезков открыта")

# ------------------------------------------------------------------ нарезка ---

$made = Start-PithExtraction $rect $fragments
Save-PithShot (Get-PithShot $rect) (Join-Path $OutDir "09_нарезка.png")
Check "отрезок вырезан" ($made -ne $null) $made

# Файл мало создать: в нём должно быть видео нужной длины.
$duration = 0.0
$hasVideo = $false
if ($made) {
    $duration = [double](& ffprobe -v error -show_entries format=duration -of csv=p=0 $made)
    $hasVideo = (& ffprobe -v error -select_streams v -show_entries stream=codec_type -of csv=p=0 $made) -match "video"
}
Check "в отрезке есть видео нужной длины" ($hasVideo -and $duration -gt 3 -and $duration -lt 8) "длительность: $([Math]::Round($duration, 1)) с при заказанных 5"

# Нажатие мимо панели: она обязана закрыться.
Click-PithAt ($rect.Left + 200) ($rect.Top + 200)
Start-Sleep -Milliseconds 700
Check "панель закрывается нажатием мимо неё" (Test-PithLog $log "панель отрезков убрана: нажали мимо неё")

# ---------------------------------------------------------------- закрытие ---

$closed = Close-PithWindow $proc 8
Check "окно закрывается без зависания" $closed "процесс завершился сам"

if (-not $closed) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}

Start-Sleep -Seconds 1

# Лог второго запуска пишется поверх первого — сохраняем этот, пока цел.
$firstLog = Join-Path $OutDir "лог_первый_запуск.log"
Copy-Item $log $firstLog -Force -ErrorAction SilentlyContinue

# ------------------------------------------- второй запуск: «продолжить» ---

$positions = Join-Path $box.Data "watch_positions.json"
Check "позиция просмотра сохранена" (Test-Path $positions)

$second = Start-Process -FilePath $box.Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait
Set-PithForeground $second | Out-Null
$secondRect = Get-PithRect $second

Save-PithShot (Get-PithShot $secondRect) (Join-Path $OutDir "10_продолжить.png")
Check "предложение продолжить показано" (Test-PithLog $log "есть сохранённая позиция просмотра")

Start-Sleep -Seconds 10
Save-PithShot (Get-PithShot $secondRect) (Join-Path $OutDir "11_продолжить_исчезло.png")
Check "предложение исчезает само" (Test-PithLog $log "предложение продолжить убрано без ответа")

$closed = Close-PithWindow $second 8
Check "второе окно закрывается без зависания" $closed
if (-not $closed) { Stop-Process -Id $second.Id -Force -ErrorAction SilentlyContinue }

# -------------------------------------------------------------------- лог ---

$errors = @(Get-PithLogErrors $firstLog) + @(Get-PithLogErrors $log)
Check "в логе нет ошибок и предупреждений" ($errors.Count -eq 0) ($errors -join "; ")

# Заметная глазу остановка интерфейса — полсекунды и больше.
$stall = [Math]::Max((Get-PithWorstStall $firstLog), (Get-PithWorstStall $log))
Check "интерфейс не замирает" ($stall -lt 500) "самая долгая заминка: $stall мс"

Copy-Item $log (Join-Path $OutDir "лог_второй_запуск.log") -Force -ErrorAction SilentlyContinue
Remove-PithSandbox $box
Remove-Item $fragments -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item Env:\PITH_LOG -ErrorAction SilentlyContinue

# ------------------------------------------------------------------ итог ---

Write-Host ""
$script:results | Format-Table -AutoSize | Out-String | Write-Host

$failed = @($script:results | Where-Object { $_.Итог -ne "ок" }).Count
Write-Host "живые проверки: $($script:results.Count), провалов: $failed"
Write-Host "снимки: $OutDir"

exit $failed
