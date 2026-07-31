# Замеры этапа 0: сравнение режимов аппаратного декодирования.
# Запуск:  powershell -File scripts\bench.ps1 -File "C:\видео.mkv"

param(
    [Parameter(Mandatory = $true)][string]$File,
    [int]$Seconds = 25,
    [string]$Exe = "target\release\pith-player.exe"
)

if (-not (Test-Path -LiteralPath $File)) {
    Write-Error "Файл не найден: $File"
    exit 1
}

$modes = @("zero-copy", "copy", "software")
$results = @()

foreach ($mode in $modes) {
    Write-Host "`n=== Режим: $mode ===" -ForegroundColor Cyan

    $log = "bench_$mode.log"
    $env:PITH_LOG = "info"

    $proc = Start-Process -FilePath $Exe `
        -ArgumentList "`"$File`"", "--hwdec=$mode" `
        -PassThru -NoNewWindow `
        -RedirectStandardOutput $log -RedirectStandardError "bench_$mode.err"

    # Даём воспроизведению выйти на установившийся режим.
    Start-Sleep -Seconds 5
    if ($proc.HasExited) {
        Write-Host "  процесс завершился досрочно (код $($proc.ExitCode))" -ForegroundColor Red
        continue
    }

    $cpuStart = $proc.CPU
    Start-Sleep -Seconds $Seconds
    $proc.Refresh()

    if ($proc.HasExited) {
        Write-Host "  процесс завершился во время замера" -ForegroundColor Red
        continue
    }

    # Доля одного ядра: сколько процессорного времени потрачено за интервал.
    $cpuUsed = $proc.CPU - $cpuStart
    $cpuPercent = [math]::Round(($cpuUsed / $Seconds) * 100, 1)
    $ramMb = [math]::Round($proc.WorkingSet64 / 1MB, 0)

    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue

    # В релизной сборке консоли нет, логи пишутся рядом с exe.
    $appLog = Join-Path (Split-Path $Exe) "pith-player.log"
    $lines = Get-Content $appLog -Encoding UTF8 -ErrorAction SilentlyContinue

    $firstFrame = "—"
    $m = $lines | Select-String -Pattern "ms=(\d+)" | Select-Object -First 1
    if ($m) { $firstFrame = $m.Matches.Groups[1].Value }

    $activeHwdec = "—"
    $h = $lines | Select-String -Pattern "hwdec_active=(\S+)" | Select-Object -First 1
    if ($h) { $activeHwdec = $h.Matches.Groups[1].Value }

    $results += [PSCustomObject]@{
        'Режим'          = $mode
        'Факт.режим'     = $activeHwdec
        'Первый кадр,мс' = $firstFrame
        'CPU,%ядра'      = $cpuPercent
        'RAM,МБ'         = $ramMb
    }

    Write-Host "  факт.режим: $activeHwdec, первый кадр: $firstFrame мс, CPU: $cpuPercent%, RAM: $ramMb МБ"
}

Write-Host "`n=== ИТОГИ ===" -ForegroundColor Green
$results | Format-Table -AutoSize
