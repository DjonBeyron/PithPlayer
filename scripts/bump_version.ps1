# Поднимает последнюю цифру версии в Cargo.toml рабочего пространства.
#
# Версия видна в заголовке окна, в правом верхнем углу и в свойствах exe,
# поэтому каждое изменение программы должно её сдвигать: по номеру сразу
# понятно, та ли это сборка, о которой идёт речь.
#
# Запуск вручную:      powershell -File scripts\bump_version.ps1
# Автоматически:       powershell -File scripts\install_hooks.ps1
#                      (ставит хук, который поднимает версию на каждый коммит)

param(
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root "Cargo.toml"

$text = [System.IO.File]::ReadAllText($manifest)

# Версия рабочего пространства: строка `version = "5.0.0"` в [workspace.package].
$pattern = '(?m)^version\s*=\s*"(\d+)\.(\d+)\.(\d+)"'
$match = [regex]::Match($text, $pattern)

if (-not $match.Success) { throw "не нашёл версию в $manifest" }

$major = [int]$match.Groups[1].Value
$minor = [int]$match.Groups[2].Value
$patch = [int]$match.Groups[3].Value + 1

$old = "$major.$minor.$($patch - 1)"
$new = "$major.$minor.$patch"

$updated = [regex]::Replace($text, $pattern, "version = `"$new`"", 1)
[System.IO.File]::WriteAllText($manifest, $updated)

# Cargo.lock хранит ту же версию для каждого крейта рабочего пространства.
$lock = Join-Path $root "Cargo.lock"
if (Test-Path $lock) {
    $lockText = [System.IO.File]::ReadAllText($lock)
    $lockText = $lockText.Replace("version = `"$old`"", "version = `"$new`"")
    [System.IO.File]::WriteAllText($lock, $lockText)
}

if (-not $Quiet) { Write-Host "версия: $old -> $new" }
$new
