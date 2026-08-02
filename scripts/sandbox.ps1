# Песочница для проверочных скриптов.
#
# Скрипты проверок ставят закладки, правят настройки и чистят данные —
# и не должны трогать данные живого пользователя в %APPDATA%\PithPlayer.
# Песочница делает временную копию плеера с файлом portable.txt: в этом
# режиме плеер хранит настройки, закладки и позиции рядом с собой
# (см. pith-store/paths.rs).
#
# Использование:
#   . "$PSScriptRoot\sandbox.ps1"
#   $box = New-PithSandbox
#   ... запускать $box.Exe вместо target\release\pith-player.exe ...
#   Remove-PithSandbox $box

function New-PithSandbox {
    param(
        [string]$Release = "target\release",
        [hashtable]$Settings = @{}
    )

    $dir = Join-Path $env:TEMP ("pith_sandbox_" + [guid]::NewGuid().ToString("N").Substring(0, 8))
    New-Item -ItemType Directory -Force $dir | Out-Null

    foreach ($file in @("pith-player.exe", "libmpv-2.dll")) {
        $path = Join-Path $Release $file
        if (-not (Test-Path $path)) { throw "нет файла: $path — соберите релиз" }
        Copy-Item $path $dir
    }

    Set-Content -Path (Join-Path $dir "portable.txt") -Encoding utf8 `
        -Value "Данные проверки. Папку можно удалять целиком."

    # Метка переноса: иначе плеер потянет в песочницу данные версии 4
    # и проверка будет идти на чужих закладках.
    #
    # Пишем через WriteAllText: `Set-Content -Encoding utf8` в PowerShell 5.1
    # добавляет метку порядка байтов, и номер стадии не разбирается.
    [System.IO.File]::WriteAllText((Join-Path $dir "migrated_from_v4"), "2")

    if ($Settings.Count -gt 0) {
        $json = $Settings | ConvertTo-Json -Depth 10
        [System.IO.File]::WriteAllText((Join-Path $dir "settings.json"), $json)
    }

    [pscustomobject]@{
        Dir  = $dir
        Exe  = Join-Path $dir "pith-player.exe"
        Data = $dir
    }
}

function Get-PithSandboxBookmarks {
    param([Parameter(Mandatory = $true)]$Sandbox)

    $file = Join-Path $Sandbox.Data "bookmarks.json"
    if (-not (Test-Path $file)) { return $null }

    Get-Content $file -Raw -Encoding UTF8 | ConvertFrom-Json
}

function Get-PithSandboxSettings {
    param([Parameter(Mandatory = $true)]$Sandbox)

    $file = Join-Path $Sandbox.Data "settings.json"
    if (-not (Test-Path $file)) { return $null }

    Get-Content $file -Raw -Encoding UTF8 | ConvertFrom-Json
}

function Remove-PithSandbox {
    param([Parameter(Mandatory = $true)]$Sandbox)

    Remove-Item -Recurse -Force $Sandbox.Dir -ErrorAction SilentlyContinue
}
