# Приёмка выпуска: всё, что должно сойтись перед отправкой в прод.
#
# Статика (формат, clippy, тесты, правила CLAUDE.md), сборка релиза
# и живые проверки собранного плеера в песочнице.
#
# Запуск:
#   powershell -File scripts\release_check.ps1 -File "видео.mkv" -Query "слово"
#   powershell -File scripts\release_check.ps1 -SkipLive   # только статика
#
# Код возврата — число проваленных проверок.

param(
    # Видео с субтитрами для живых проверок.
    [string]$File = "",
    # Слово из субтитров этого видео — для проверки поиска.
    [string]$Query = "",
    [switch]$SkipLive,
    [string]$OutDir = "release_check"
)

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
. "$PSScriptRoot\ui_probe.ps1"

$script:results = @()

function Check([string]$name, [bool]$ok, [string]$note = "") {
    $script:results += [pscustomobject]@{ Проверка = $name; Итог = $(if ($ok) { "ок" } else { "ПРОВАЛ" }); Пояснение = $note }
    $mark = $(if ($ok) { "  ок  " } else { "ПРОВАЛ" })
    Write-Host "[$mark] $name $note"
}

function Invoke-Cargo([string]$name, [string[]]$cargoArgs) {
    $output = & cargo @cargoArgs 2>&1
    $ok = $LASTEXITCODE -eq 0
    $note = ""
    if (-not $ok) {
        $note = ($output | Select-String -Pattern "^error" | Select-Object -First 3) -join " / "
    }
    Check $name $ok $note
    return $output
}

Write-Host "=== Статика ==="

Invoke-Cargo "формат исходников" @("fmt", "--check") | Out-Null
Invoke-Cargo "clippy без предупреждений" @("clippy", "--all-targets", "--", "-D", "warnings") | Out-Null

$tests = Invoke-Cargo "тесты" @("test")
$passed = ($tests | Select-String -Pattern "(\d+) passed" -AllMatches |
    ForEach-Object { $_.Matches } | ForEach-Object { [int]$_.Groups[1].Value } |
    Measure-Object -Sum).Sum
Write-Host "        тестов пройдено: $passed"

# --- правила CLAUDE.md ---

# Переменную цикла нельзя звать `$file`: так называется параметр скрипта,
# а он объявлен строкой — PowerShell молча приводит к ней любой объект,
# и проверки читали бы пустоту.
$sources = Get-ChildItem crates -Recurse -Filter *.rs | Where-Object { $_.FullName -notmatch "\\target\\" }

$tooLong = @()
foreach ($item in $sources) {
    $lines = (Get-Content $item.FullName | Measure-Object -Line).Lines
    if ($lines -gt 400) { $tooLong += "$($item.Name): $lines" }
}
Check "предел 400 строк на файл" ($tooLong.Count -eq 0) ($tooLong -join ", ")

# `unwrap`, `expect`, `println!` и `dbg!` разрешены только в тестах.
#
# Мимо проверки идут: сборочные скрипты (`println!` — это язык общения
# с cargo, другого у них нет), примеры и строки, помеченные `// разрешено:`
# с объяснением. Всё остальное — нарушение правила из CLAUDE.md.
$forbidden = @()
foreach ($item in $sources) {
    if ($item.FullName -match "\\examples\\|\\tests\\") { continue }
    if ($item.Name -eq "build.rs") { continue }

    $inTests = $false
    $allowed = $false
    $number = 0
    # Кодировку задаём явно: без неё PowerShell 5.1 читает файл как ANSI,
    # и русские комментарии — а с ними и пометки — превращаются в кашу.
    foreach ($line in Get-Content $item.FullName -Encoding UTF8) {
        $number++
        if ($line -match "#\[cfg\(test\)\]") { $inTests = $true }
        if ($inTests) { continue }

        # Пометка стоит либо в самой строке, либо в комментарии над ней:
        # rustfmt переносит длинные вызовы, и хвост строки не удержать.
        if ($line -match "// разрешено:") { $allowed = $true; continue }

        if ($line -match "^\s*//") { continue }

        if ($line -match "\.unwrap\(\)|\.expect\(|println!|dbg!") {
            if (-not $allowed) { $forbidden += "$($item.Name):$number" }
        }

        $allowed = $false
    }
}
Check "нет unwrap/expect/println в рабочем коде" ($forbidden.Count -eq 0) ($forbidden -join ", ")

Write-Host ""
Write-Host "=== Сборка выпуска ==="

Invoke-Cargo "сборка релиза" @("build", "--release") | Out-Null

$exe = "target\release\pith-player.exe"
$dll = "target\release\libmpv-2.dll"

Check "собран pith-player.exe" (Test-Path $exe)
Check "рядом лежит libmpv-2.dll" (Test-Path $dll) "без неё плеер не запустится"

if (Test-Path $exe) {
    $info = [System.Diagnostics.FileVersionInfo]::GetVersionInfo((Resolve-Path $exe))
    $cargoVersion = (Select-String -Path "Cargo.toml" -Pattern '^version = "(.+)"' |
        Select-Object -First 1).Matches[0].Groups[1].Value
    $same = $info.FileVersion -like "$cargoVersion*"
    Check "версия в exe совпадает с Cargo.toml" $same "exe: $($info.FileVersion), Cargo: $cargoVersion"
    Check "в exe есть название продукта" ($info.ProductName -eq "Pith Player") $info.ProductName
}

# Плеер обязан работать и без ffmpeg: без него отключается только нарезка.
$ffmpeg = $null -ne (Get-Command ffmpeg -ErrorAction SilentlyContinue)
Write-Host "        ffmpeg в PATH: $ffmpeg"

if (-not $SkipLive) {
    Write-Host ""
    Write-Host "=== Живые проверки ==="

    if ($File -eq "") {
        Check "живые проверки" $false "не задан -File с видео"
    } else {
        & powershell -NoProfile -File "$PSScriptRoot\live_check.ps1" -File $File -Query $Query -OutDir $OutDir
        $liveFailed = $LASTEXITCODE
        Check "живые проверки плеера" ($liveFailed -eq 0) "провалов: $liveFailed"
    }
}

Write-Host ""
Write-Host "=== Поставка ==="

# Собираем ровно то, что уедет пользователю, и смотрим, всё ли на месте.
& powershell -NoProfile -File "$PSScriptRoot\package.ps1" -SkipBuild | Out-Null
Check "поставка собирается" ($LASTEXITCODE -eq 0)

$zip = Get-ChildItem "dist" -Filter "PithPlayer-*-portable.zip" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1

Check "ZIP поставки создан" ($null -ne $zip) $(if ($zip) { "$($zip.Name), $([Math]::Round($zip.Length / 1MB, 1)) МБ" })

if ($zip) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::OpenRead($zip.FullName)
    $names = $archive.Entries | ForEach-Object { $_.Name }
    $archive.Dispose()

    $needed = @("pith-player.exe", "libmpv-2.dll")
    $missing = @($needed | Where-Object { $names -notcontains $_ })
    Check "в поставке есть всё нужное" ($missing.Count -eq 0) "внутри: $($names -join ', ')"
}

Write-Host ""
Write-Host "=== Итог ==="
$script:results | Format-Table -AutoSize | Out-String | Write-Host

$failed = @($script:results | Where-Object { $_.Итог -ne "ок" }).Count

if ($failed -eq 0) {
    Write-Host "Всё сошлось: можно выпускать."
} else {
    Write-Host "Провалов: $failed — выпускать нельзя."
}

exit $failed
