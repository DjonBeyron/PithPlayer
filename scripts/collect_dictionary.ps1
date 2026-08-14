# Собирает словарь транскрипций, который едет в поставке.
#
# Слова копятся у пользователя в %APPDATA%\PithPlayer\transcriptions.json,
# пока он смотрит кино. Этот скрипт переносит их в файл, вложенный
# в программу: свежая установка стартует не с пустого словаря, а с готовым,
# и первая же выгрузка идёт без похода в сеть.
#
# Запуск перед выпуском:
#   powershell -File scripts\collect_dictionary.ps1
#
# Слияние, а не замена: слова, собранные на другой машине и уже лежащие
# в поставке, никуда не деваются.

param(
    # Откуда брать. По умолчанию — словарь этого компьютера.
    [string]$From = "$env:APPDATA\PithPlayer\transcriptions.json"
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$target = Join-Path $root "crates\pith-app\assets\dictionary.json"

if (-not (Test-Path $From)) { throw "нет словаря: $From" }

$mine = Get-Content $From -Raw -Encoding UTF8 | ConvertFrom-Json

$words = [ordered]@{}

# Сперва то, что уже в поставке, — его мы только дополняем.
if (Test-Path $target) {
    $packed = Get-Content $target -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($p in $packed.words.PSObject.Properties) { $words[$p.Name] = $p.Value }
}

$before = $words.Count

foreach ($p in $mine.words.PSObject.Properties) {
    if (-not $words.Contains($p.Name)) { $words[$p.Name] = $p.Value }
}

# По алфавиту: файл лежит в репозитории, и его читают глазами — а ещё
# так его правки в истории видно построчно, а не одной перетасовкой.
$sorted = [ordered]@{}
foreach ($key in ($words.Keys | Sort-Object)) { $sorted[$key] = $words[$key] }

$out = [ordered]@{ version = 1; words = $sorted }
$json = $out | ConvertTo-Json -Depth 6

[System.IO.File]::WriteAllText($target, $json, (New-Object System.Text.UTF8Encoding($false)))

$added = $sorted.Count - $before
Write-Host "словарь поставки: было $before, прибавилось $added, стало $($sorted.Count)"
Write-Host "файл: $target ($([math]::Round((Get-Item $target).Length / 1KB, 1)) КБ)"
