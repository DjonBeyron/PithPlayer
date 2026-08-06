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
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
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

# FFmpeg не обязателен: без него не работает только нарезка отрезков.
foreach ($tool in @("ffmpeg.exe", "ffprobe.exe")) {
    $found = (Get-Command $tool -ErrorAction SilentlyContinue).Source
    if ($found) {
        Copy-Item $found $staging
        Write-Host "вложен $tool"
    } else {
        Write-Warning "$tool не найден в PATH — нарезка в установленной программе будет недоступна"
    }
}

& $iscc "/DVersion=$version" "/DStaging=$((Resolve-Path $staging).Path)" "installer\pith-player.iss"
if ($LASTEXITCODE -ne 0) { throw "Inno Setup вернул ошибку" }

$setup = Join-Path $OutputDir "PithPlayer-$version-setup.exe"
$size = [math]::Round((Get-Item $setup).Length / 1MB, 1)
Write-Host "`nготово: $setup ($size МБ)"
