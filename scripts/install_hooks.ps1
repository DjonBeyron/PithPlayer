# Ставит git-хук, который поднимает последнюю цифру версии на каждый коммит.
#
# Запуск: powershell -File scripts\install_hooks.ps1
#
# Хук живёт в .git\hooks и в репозиторий не попадает — поэтому его нужно
# поставить один раз на каждой машине, где ведётся разработка.

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$hooks = Join-Path $root ".git\hooks"

if (-not (Test-Path $hooks)) { throw "нет каталога $hooks — это точно репозиторий?" }

$hook = @'
#!/bin/sh
# Поднимает последнюю цифру версии перед каждым коммитом.
# Поставлен scripts/install_hooks.ps1.

# Пропускаем, когда коммит правит только версию: иначе `git commit --amend`
# и правки хука сдвигали бы номер снова и снова.
if git diff --cached --name-only | grep -qv -e '^Cargo.toml$' -e '^Cargo.lock$'; then
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/bump_version.ps1 -Quiet > /dev/null
    git add Cargo.toml Cargo.lock
fi
'@

$path = Join-Path $hooks "pre-commit"
[System.IO.File]::WriteAllText($path, ($hook -replace "`r`n", "`n"))

Write-Host "хук поставлен: $path"
Write-Host "теперь каждый коммит поднимает последнюю цифру версии"
