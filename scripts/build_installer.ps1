# Собирает установщик Pith Player (PithPlayer-<версия>-setup.exe).
#
# Запуск:
#   powershell -File scripts\build_installer.ps1
#   powershell -File scripts\build_installer.ps1 -SkipBuild
#
# Нужен Inno Setup 6: https://jrsoftware.org/isdl.php
# Портативный ZIP собирается отдельно — scripts\package.ps1.

param(
    [switch]$SkipBuild,
    # Вложить FFmpeg в установщик вместо загрузки при установке.
    # Нужно для поставки в сеть, где скачивать нечего.
    [switch]$WithFfmpeg,
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

# Компилятор Inno Setup: сначала обычные места установки, потом PATH.
$candidates = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    # winget ставит Inno Setup в профиль пользователя, если нет прав
    # администратора — там его и находим чаще всего.
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
)
$iscc = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { $iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source }
if (-not $iscc) {
    throw "не найден ISCC.exe. Поставьте Inno Setup 6 (https://jrsoftware.org/isdl.php) и повторите"
}

if (-not $SkipBuild) {
    Write-Host "собираю релиз…"
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "сборка не удалась" }
}

# Всё, что попадёт в установщик, сначала складываем в одну папку: так
# список файлов в .iss остаётся одной строкой и не расходится со сборкой.
$release = Join-Path $root "target\release"
$staging = Join-Path $OutputDir "installer-staging"

Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $staging | Out-Null

foreach ($file in @("pith-player.exe", "libmpv-2.dll")) {
    $path = Join-Path $release $file
    if (-not (Test-Path $path)) { throw "нет файла: $path" }
    Copy-Item $path $staging
}

# Проверяет, что вложенный FFmpeg работает сам по себе.
#
# В PATH часто оказывается не сам ffmpeg, а перенаправляющий ярлык —
# шим Chocolatey или scoop весом в треть мегабайта. У нас в поставку
# однажды уехал именно такой: на чужой машине он ищет несуществующий
# путь и нарезка не работает вовсе. Проверяем запуском из пустой папки:
# там от шима остаётся только ошибка.
function Assert-Standalone {
    param([string]$Directory)

    $probe = Join-Path $env:TEMP ("pith_ffmpeg_probe_" + [guid]::NewGuid().ToString("N").Substring(0, 8))
    New-Item -ItemType Directory -Force $probe | Out-Null

    try {
        foreach ($tool in @("ffmpeg.exe", "ffprobe.exe")) {
            Copy-Item (Join-Path $Directory $tool) $probe
        }

        foreach ($tool in @("ffmpeg.exe", "ffprobe.exe")) {
            & (Join-Path $probe $tool) -version > $null 2>&1
            if ($LASTEXITCODE -ne 0) {
                throw "$tool не работает сам по себе — в PATH лежит перенаправляющий ярлык, а не FFmpeg. Соберите без -WithFfmpeg: установщик скачает настоящий."
            }
        }

        Write-Host "вложенный FFmpeg проверен: работает без своей папки"
    } finally {
        Remove-Item -Recurse -Force $probe -ErrorAction SilentlyContinue
    }
}

# По умолчанию FFmpeg не вкладывается: установщик предложит скачать его
# сам — свежую сборку и без лишних сорока мегабайт в самом файле.
$arguments = @("/DVersion=$version", "/DStaging=$((Resolve-Path $staging).Path)")

if ($WithFfmpeg) {
    foreach ($tool in @("ffmpeg.exe", "ffprobe.exe")) {
        $found = (Get-Command $tool -ErrorAction SilentlyContinue).Source
        if (-not $found) { throw "$tool не найден в PATH — вложить нечего" }

        Copy-Item $found $staging
        Write-Host "вложен $tool"
    }

    Assert-Standalone $staging
    $arguments += "/DBundledFfmpeg=1"
} else {
    Write-Host "FFmpeg скачается при установке"
}

& $iscc @arguments "installer\pith-player.iss"
if ($LASTEXITCODE -ne 0) { throw "Inno Setup вернул ошибку" }

$setup = Join-Path $OutputDir "PithPlayer-$version-setup.exe"
$size = [math]::Round((Get-Item $setup).Length / 1MB, 1)
Write-Host "`nготово: $setup ($size МБ)"
