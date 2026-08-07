# Замер открытия файла: от старта процесса до первого показанного кадра.
#
# Запуск:
#   powershell -File scripts\open_bench.ps1 -Files "C:\видео.mkv" -Runs 3
#
# Работает в песочнице (portable.txt рядом с копией плеера): настройки,
# закладки и позиции просмотра живого пользователя не трогаются.
#
# Все отрезки считаются по меткам времени в журнале и по времени старта
# процесса — секундомер оболочки в счёт не идёт: Start-Process возвращает
# управление с задержкой в сотни миллисекунд, и она приписывалась бы плееру.

param(
    [Parameter(Mandatory = $true)][string[]]$Files,
    [int]$Runs = 3,
    [int]$TimeoutSeconds = 90
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

. "$PSScriptRoot\sandbox.ps1"

# Вехи: подпись в отчёте и кусок строки журнала, по которому она ищется.
$marks = @(
    @{ Name = "запуск процесса";    Pattern = "переносной режим" },
    @{ Name = "проверка запуска";   Pattern = "стал основным|файл передан" },
    @{ Name = "окно и шрифты";      Pattern = "шрифт значков" },
    @{ Name = "движок mpv";         Pattern = "движок mpv создан" },
    @{ Name = "контекст отрисовки"; Pattern = "контекст отрисовки готов" },
    @{ Name = "загрузка файла";     Pattern = "файл загружен" },
    @{ Name = "первый кадр";        Pattern = "время до первого кадра" }
)

function Get-LogTime {
    param([string[]]$Lines, [string]$Pattern)

    foreach ($line in $Lines) {
        if ($line -match $Pattern -and $line -match '^(\S+Z)') {
            return [datetime]::Parse($matches[1], $null, [Globalization.DateTimeStyles]::RoundtripKind)
        }
    }
    return $null
}

$box = New-PithSandbox
$log = Join-Path (Split-Path $box.Exe) "pith-player.log"
$results = @()

try {
    foreach ($file in $Files) {
        if (-not (Test-Path -LiteralPath $file)) { throw "нет файла: $file" }

        $size = [math]::Round((Get-Item -LiteralPath $file).Length / 1MB, 1)
        Write-Host "`n=== $(Split-Path $file -Leaf) ($size МБ) ===" -ForegroundColor Cyan

        for ($run = 1; $run -le $Runs; $run++) {
            Remove-Item $log -Force -ErrorAction SilentlyContinue

            $watch = [Diagnostics.Stopwatch]::StartNew()
            $proc = Start-Process -FilePath $box.Exe -ArgumentList "`"$file`"" -PassThru
            $startedAt = $proc.StartTime.ToUniversalTime()

            while ($watch.Elapsed.TotalSeconds -lt $TimeoutSeconds) {
                if ((Test-Path $log) -and
                    (Select-String -Path $log -Pattern "время до первого кадра" -Quiet)) { break }
                Start-Sleep -Milliseconds 20
            }

            $lines = Get-Content $log -Encoding UTF8 -ErrorAction SilentlyContinue

            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            # Пока процесс жив, файл журнала занят, и следующий прогон
            # остался бы вовсе без записей.
            $proc.WaitForExit(5000) | Out-Null

            # Разбивка: сколько прошло до каждой вехи от предыдущей.
            $stages = [ordered]@{}
            $previous = $startedAt
            $frameAt = $null

            foreach ($mark in $marks) {
                $time = Get-LogTime -Lines $lines -Pattern $mark.Pattern
                if ($null -eq $time) { continue }
                $stages[$mark.Name] = [math]::Round(($time - $previous).TotalMilliseconds)
                $previous = $time
                $frameAt = $time
            }

            $total = if ($frameAt) {
                [math]::Round(($frameAt - $startedAt).TotalMilliseconds)
            } else {
                "—"
            }

            $results += [PSCustomObject]@{
                'Файл'   = Split-Path $file -Leaf
                'Прогон' = $run
                'Всего'  = $total
            }

            Write-Host ("  прогон {0}: всего {1} мс" -f $run, $total)
            foreach ($stage in $stages.GetEnumerator()) {
                Write-Host ("      {0,-22} {1,6} мс" -f $stage.Key, $stage.Value)
            }
        }
    }
} finally {
    Remove-PithSandbox $box
}

Write-Host "`n=== ИТОГИ ===" -ForegroundColor Green
$results | Format-Table -AutoSize
