# Проверяет, что плеер не деградирует при переключении файлов подряд
# (PLAN.md, чек-лист этапа 6).
#
# Файлы открываются так же, как из проводника: повторный запуск exe
# передаёт путь работающему экземпляру и выходит. После каждого файла
# снимается рабочий набор памяти и число дескрипторов — по ним видно
# утечку, если она есть.
#
# Запуск: powershell -File scripts\soak_test.ps1 -Folder "C:\видео" -Count 10

param(
    [Parameter(Mandatory = $true)][string]$Folder,
    [int]$Count = 10,
    [int]$SecondsPerFile = 4,
    [string]$Release = "target\release"
)

$ErrorActionPreference = "Stop"

# Песочница: позиции просмотра живого пользователя не затрагиваются.
. "$PSScriptRoot\sandbox.ps1"
$box = New-PithSandbox -Release $Release

$files = Get-ChildItem -LiteralPath $Folder -File |
    Where-Object { $_.Extension -in ".mkv", ".mp4", ".avi", ".mov", ".webm" } |
    Select-Object -First $Count

if ($files.Count -lt 2) { throw "нужно хотя бы два видеофайла в $Folder" }
Write-Host "файлов в прогоне: $($files.Count)"

$proc = Start-Process -FilePath $box.Exe -ArgumentList "`"$($files[0].FullName)`"" -PassThru
Start-Sleep -Seconds 6

$rows = @()
foreach ($file in $files) {
    # Первый файл уже открыт запуском выше.
    if ($file -ne $files[0]) {
        Start-Process -FilePath $box.Exe -ArgumentList "`"$($file.FullName)`"" | Out-Null
    }
    Start-Sleep -Seconds $SecondsPerFile

    $proc.Refresh()
    if ($proc.HasExited) {
        Write-Host "ПРОВАЛ: плеер завершился на файле $($file.Name)" -ForegroundColor Red
        exit 1
    }

    $rows += [pscustomobject]@{
        Файл          = $file.Name
        ПамятьМБ      = [math]::Round($proc.WorkingSet64 / 1MB, 1)
        Дескрипторы   = $proc.HandleCount
        Потоки        = $proc.Threads.Count
        Отвечает      = $proc.Responding
    }
}

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Remove-PithSandbox $box

$rows | Format-Table -AutoSize

$first = $rows[0].ПамятьМБ
$last = $rows[-1].ПамятьМБ
$growth = [math]::Round($last - $first, 1)

Write-Host "память: $first МБ в начале, $last МБ в конце (рост $growth МБ)"
Write-Host "дескрипторы: $($rows[0].Дескрипторы) → $($rows[-1].Дескрипторы)"

# Порог грубый: он ловит явную утечку, а не колебания кэша демуксера.
if ($growth -gt 300) {
    Write-Host "ПОХОЖЕ НА УТЕЧКУ: рост больше 300 МБ" -ForegroundColor Red
} else {
    Write-Host "рост в пределах разумного" -ForegroundColor Green
}
