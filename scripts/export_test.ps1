# Живая проверка окна выгрузки в Notion.
#
# Ставит две закладки, выдвигает панель отрезков, нажимает «Выгрузить
# в Notion» и снимает окно с вопросом. В Notion при этом не ходит:
# кнопку «Выгрузить» скрипт не нажимает.
#
# Запуск: powershell -File scripts\export_test.ps1 -File "видео.mp4"

param(
    [Parameter(Mandatory = $true)][string]$File,
    [string]$Out = "shot_export.png",
    [int]$Wait = 7,
    # Настройки Notion в песочнице: без них кнопка открывает окно
    # интеграций, а не вопрос о названии.
    [switch]$NoNotion,
    # Настоящая выгрузка: токен берётся у синхронизатора, страница —
    # из -WorkPage, и скрипт жмёт «Выгрузить». В рабочей базе появятся
    # настоящие строки.
    [switch]$Real,
    [string]$WorkPage = "",
    # Положить в песочницу готовые закладки — с репликой и актёром.
    # Без него закладки ставятся клавишей и уходят безымянными.
    [switch]$Seed
)

. "$PSScriptRoot\ui_probe.ps1"
. "$PSScriptRoot\sandbox.ps1"

function Resolve-PithOut([string]$path) {
    if ([System.IO.Path]::IsPathRooted($path)) { return $path }
    return (Join-Path $PWD $path)
}

# Токен настоящий не нужен: до сети дело не доходит, а поля должны быть
# заполнены — иначе кнопка уводит в окно интеграций.
$settings = @{}
if (-not $NoNotion) {
    $token = "ntn_проверка"
    $work = "https://app.notion.com/p/Cards-prod-3bab5e539287804d83b7f934db040493"

    if ($Real) {
        if (-not $WorkPage) { throw "нужна -WorkPage: страница с рабочей базой" }

        $config = "C:\PITH\Development\NOTION_PITH\NotionSync\config.json"
        if (-not (Test-Path $config)) { throw "нет настроек синхронизатора: $config" }

        $token = (Get-Content $config -Raw -Encoding UTF8 | ConvertFrom-Json).NOTION_TOKEN
        $work = $WorkPage
    }

    $settings = @{
        notion = @{
            token         = $token
            work_page     = $work
            template_page = "https://app.notion.com/p/DIFF-330b5e5392878039ab95ef453be3db03"
        }
    }
}

$box = New-PithSandbox -Settings $settings

# Готовые закладки: ключ видео — имя файла без расширения.
if ($Seed) {
    $key = [System.IO.Path]::GetFileNameWithoutExtension($File)
    $data = @{
        version = 2
        videos  = @(@{
                video_file_name = $key
                active_list     = "Основной"
                lists           = @(@{
                        name         = "Основной"
                        duration_sec = 18
                        buffer_sec   = 5
                        output_dir   = $null
                        bookmarks    = @(
                            @{ time_ms = 3000; name = "That explains a lot."; actor = "Лили Коллинз (Emily)" },
                            @{ time_ms = 8000; name = "I cannot believe you."; actor = $null }
                        )
                    })
            })
    }

    [System.IO.File]::WriteAllText(
        (Join-Path $box.Data "bookmarks.json"),
        ($data | ConvertTo-Json -Depth 10),
        [System.Text.Encoding]::UTF8)
}

$proc = Start-Process -FilePath $box.Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

try {
    if (-not (Set-PithForeground $proc)) { throw "окно плеера не вышло вперёд" }

    $rect = Get-PithRect $proc
    $midY = [int](($rect.Top + $rect.Bottom) / 2)

    # Две закладки: список должен быть непустым, иначе кнопок нарезки нет.
    if (-not $Seed) {
        Send-PithKey 't'
        Send-PithKey 'right'
        Start-Sleep -Milliseconds 500
        Send-PithKey 't'
        Start-Sleep -Milliseconds 500
    }

    # Панель вызывается язычком у правого края: он появляется, когда
    # курсор доходит до правой пятой части окна, и открывает панель
    # нажатием.
    for ($x = ($rect.Right - 300); $x -lt ($rect.Right - 15); $x += 20) {
        Set-PithCursor $x $midY
        Start-Sleep -Milliseconds 40
    }
    Start-Sleep -Milliseconds 500
    Click-PithAt ($rect.Right - 9) $midY
    Start-Sleep -Seconds 1

    # Снимок выдвинутой панели: по нему видно, куда встали кнопки.
    $panel = Get-PithShot $rect
    Save-PithShot $panel (Resolve-PithOut ($Out -replace '\.png$', '_panel.png'))
    $panel.Dispose()

    # Кнопки стоят одной строкой у нижнего края панели: «Вырезать
    # отрезки», за ней значок выгрузки и значок очистки. Значки шириной
    # 38 прижаты к правому краю панели, панель — к правому краю окна.
    $exportY = $rect.Bottom - 82
    Click-PithAt ($rect.Right - 85) $exportY
    Start-Sleep -Seconds 2

    # Окно выгрузки — отдельное окно системы, а не карточка в кадре.
    $window = Get-PithChildRect $proc "Выгрузка в Notion"
    if (-not $window) { throw "окно выгрузки не появилось" }

    $shot = Get-PithShot $window
    Save-PithShot $shot (Resolve-PithOut ($Out -replace '\.png$', '_window.png'))
    $shot.Dispose()

    # Настоящая выгрузка: кнопка «Выгрузить» стоит в нижней строке окна.
    if ($Real) {
        Click-PithAt ($window.Left + 50) ($window.Bottom - 30)

        # Строка за строкой, каждая — запрос к Notion.
        Start-Sleep -Seconds 25
    }

    $last = Get-PithChildRect $proc "Выгрузка в Notion"
    if (-not $last) { $last = $rect }

    $shot = Get-PithShot $last
    Save-PithShot $shot (Resolve-PithOut $Out)
    $shot.Dispose()
    Write-Host "снимок: $Out"

    $bookmarks = Get-PithSandboxBookmarks $box
    if ($bookmarks) {
        $count = 0
        foreach ($video in $bookmarks.videos) {
            foreach ($list in $video.lists) { $count += @($list.bookmarks).Count }
        }
        Write-Host "закладок в песочнице: $count"
    }
}
finally {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 400
    Remove-PithSandbox $box
}
