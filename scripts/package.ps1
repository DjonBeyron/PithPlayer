# Собирает портативную поставку Pith Player: ZIP, который достаточно
# распаковать (PLAN.md, этап 6).
#
# Запуск:
#   powershell -File scripts\package.ps1
#   powershell -File scripts\package.ps1 -WithFfmpeg   # вложить FFmpeg
#
# По умолчанию данные пользователя остаются в %APPDATA%\PithPlayer:
# так они переживают замену папки на новую версию. Ключ -Portable кладёт
# рядом с программой файл portable.txt, и тогда всё хранится в самой папке —
# годится для флешки.

param(
    [switch]$WithFfmpeg,
    [switch]$Portable,
    [switch]$SkipBuild,
    [string]$OutputDir = "dist"
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# Версия — единственный источник истины в Cargo.toml рабочего пространства.
$version = (Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"(.+)"' |
    Select-Object -First 1).Matches.Groups[1].Value
if (-not $version) { throw "не удалось прочитать версию из Cargo.toml" }

Write-Host "версия: $version"

if (-not $SkipBuild) {
    Write-Host "собираю релиз…"
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "сборка не удалась" }
}

$release = Join-Path $root "target\release"
$name = "PithPlayer-$version-portable"
$staging = Join-Path $OutputDir $name

Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $staging | Out-Null

# Обязательное: сам плеер и libmpv. Без DLL программа не запустится.
$required = @("pith-player.exe", "libmpv-2.dll")
foreach ($file in $required) {
    $path = Join-Path $release $file
    if (-not (Test-Path $path)) { throw "нет файла: $path" }
    Copy-Item $path $staging
}

if ($WithFfmpeg) {
    foreach ($tool in @("ffmpeg.exe", "ffprobe.exe")) {
        $found = (Get-Command $tool -ErrorAction SilentlyContinue).Source
        if ($found) {
            Copy-Item $found $staging
            Write-Host "вложен $tool"
        } else {
            Write-Warning "$tool не найден в PATH — нарезка будет недоступна"
        }
    }
}

if ($Portable) {
    Set-Content -Path (Join-Path $staging "portable.txt") -Encoding utf8 -Value @"
Признак переносного режима.

Пока этот файл лежит рядом с pith-player.exe, настройки, закладки
и позиции просмотра хранятся здесь же, а не в профиле пользователя.
Удалите файл, чтобы вернуться к %APPDATA%\PithPlayer.
"@
}

$readme = @"
Pith Player $version

Запуск: pith-player.exe. Установка не нужна, папку можно переносить целиком.

Состав:
  pith-player.exe  — плеер
  libmpv-2.dll     — движок воспроизведения, обязателен

Нарезка отрезков требует ffmpeg.exe и ffprobe.exe: положите их в эту папку
либо установите FFmpeg так, чтобы он был доступен в PATH. Без них плеер
работает как обычно, недоступна только нарезка.

Данные (настройки, закладки, позиции просмотра) хранятся в
%APPDATA%\PithPlayer. Если рядом с программой лежит portable.txt,
они хранятся в самой папке.

Горячие клавиши: пробел — пауза, F — полный экран, стрелки — перемотка
и громкость, T — закладка, C — скопировать реплику, Ctrl+F — поиск
по субтитрам. Правый щелчок по видео открывает меню.
"@

Set-Content -Path (Join-Path $staging "ЧИТАЙ МЕНЯ.txt") -Value $readme -Encoding utf8

$zip = Join-Path $OutputDir "$name.zip"
Remove-Item -Force $zip -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $staging "*") -DestinationPath $zip

$size = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Host "`nготово: $zip ($size МБ)"
Get-ChildItem $staging | ForEach-Object {
    "  {0,-20} {1,8:N0} КБ" -f $_.Name, ($_.Length / 1KB)
}
