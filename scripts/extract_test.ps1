# Проверка нарезки: ставит закладки клавишей T и запускает вырезание.
#
# Запуск: powershell -File scripts\extract_test.ps1 -File "видео.mkv"

param(
    [Parameter(Mandatory = $true)][string]$File,
    [int]$Wait = 45,
    [string]$Exe = "target\release\pith-player.exe"
)

Add-Type -AssemblyName System.Windows.Forms

$source = @"
using System;
using System.Runtime.InteropServices;
public class ExtractTest {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
"@
if (-not ("ExtractTest" -as [type])) { Add-Type -TypeDefinition $source }

# Чистим вывод прошлого прогона.
$outDir = "$env:TEMP\pith_fragments_test"
Remove-Item $outDir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

# Направляем нарезку во временную папку и ставим короткие отрезки.
$settingsPath = "$env:APPDATA\PithPlayer\settings.json"
$settings = Get-Content $settingsPath -Raw -Encoding UTF8 | ConvertFrom-Json
$settings.fragments.output_dir = $outDir
$settings.fragments.duration_sec = 5
$settings.fragments.buffer_sec = 2
[System.IO.File]::WriteAllText($settingsPath, ($settings | ConvertTo-Json -Depth 10))

# Закладки текущего видео убираем, чтобы считать только новые.
Remove-Item "$env:APPDATA\PithPlayer\bookmarks.json" -Force -ErrorAction SilentlyContinue
Remove-Item "$env:APPDATA\PithPlayer\watch_positions.json" -Force -ErrorAction SilentlyContinue

$proc = Start-Process -FilePath $Exe -ArgumentList "`"$File`"" -PassThru
Start-Sleep -Seconds $Wait

for ($i = 0; $i -lt 10; $i++) {
    [ExtractTest]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 400
    if ([ExtractTest]::GetForegroundWindow() -eq $proc.MainWindowHandle) { break }
}

Write-Host "ставлю три закладки"
foreach ($n in 1..3) {
    [System.Windows.Forms.SendKeys]::SendWait("t")
    Start-Sleep -Seconds 3
}

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 800

$bookmarks = Get-Content "$env:APPDATA\PithPlayer\bookmarks.json" -Raw -Encoding UTF8 -ErrorAction SilentlyContinue
if ($bookmarks) {
    $data = $bookmarks | ConvertFrom-Json
    $count = ($data.videos | ForEach-Object { $_.lists[0].bookmarks.Count } | Measure-Object -Sum).Sum
    Write-Host "закладок сохранено: $count"
} else {
    Write-Host "закладки не сохранились"
}

Write-Host "папка вывода: $outDir"
