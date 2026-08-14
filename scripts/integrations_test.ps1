# Живая проверка окна интеграций.
#
# Открывает контекстное меню, нажимает «Интеграции…», снимает окно
# и сверяет, что настройки Notion легли в settings.json песочницы.
#
# Запуск: powershell -File scripts\integrations_test.ps1
#
# Ключ -MenuOnly снимает только раскрытое меню: по снимку берутся
# координаты пункта, они задаются ключом -ItemY (от верха окна).

param(
    [string]$Out = "shot_integrations.png",
    [int]$Wait = 6,
    [int]$ItemY = 0,
    [switch]$MenuOnly,
    # Нажать «Проверить доступ» и «Сохранить». Проверка ходит в Notion
    # по-настоящему, но только читает страницы.
    [switch]$Check
)

. "$PSScriptRoot\ui_probe.ps1"
. "$PSScriptRoot\sandbox.ps1"

Add-Type -AssemblyName System.Windows.Forms

# Имя снимка: относительное — от текущей папки, полное — как есть.
function Resolve-PithOut([string]$path) {
    if ([System.IO.Path]::IsPathRooted($path)) { return $path }
    return (Join-Path $PWD $path)
}

$rightClick = @"
using System;
using System.Runtime.InteropServices;
public class PithRight {
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, int e);
    public static void Click() {
        mouse_event(0x0008, 0, 0, 0, 0);
        System.Threading.Thread.Sleep(60);
        mouse_event(0x0010, 0, 0, 0, 0);
    }
}
"@
if (-not ("PithRight" -as [type])) { Add-Type -TypeDefinition $rightClick }

$box = New-PithSandbox
$proc = Start-Process -FilePath $box.Exe -PassThru
Start-Sleep -Seconds $Wait
$proc.Refresh()

try {
    if (-not (Set-PithForeground $proc)) { throw "окно плеера не вышло вперёд" }

    $rect = Get-PithRect $proc
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top

    # Щёлкаем слева сверху: меню длинное и должно уместиться вниз.
    $x = [int]($rect.Left + $width * 0.20)
    $y = [int]($rect.Top + $height * 0.10)

    Move-PithCursorTo $x $y
    Start-Sleep -Milliseconds 300
    [PithRight]::Click()
    Start-Sleep -Milliseconds 900

    if ($MenuOnly) {
        $shot = Get-PithShot $rect
        Save-PithShot $shot (Resolve-PithOut $Out)
        $shot.Dispose()
        Write-Host "снимок меню: $Out (окно $width x $height, щелчок в $($x - $rect.Left), $($y - $rect.Top))"
        return
    }

    if ($ItemY -le 0) { throw "нужен -ItemY: возьмите его со снимка -MenuOnly" }

    Click-PithAt ($x + 60) ($rect.Top + $ItemY)
    Start-Sleep -Milliseconds 1200

    # Окно интеграций — своё окно процесса, и оно становится главным:
    # снимаем именно его.
    $proc.Refresh()
    $window = Get-PithRect $proc
    Write-Host "окно: $($window.Right - $window.Left) x $($window.Bottom - $window.Top)"

    if ($Check) {
        if (-not (Set-PithForeground $proc)) { throw "окно интеграций не вышло вперёд" }

        # Кнопки стоят на своих местах в окне: проверка доступа под полями,
        # «Сохранить» — в нижней строке.
        Click-PithAt ($window.Left + 80) ($window.Top + 289)
        Start-Sleep -Seconds 6
    }

    # Снимок делается до «Сохранить»: иначе итог проверки доступа
    # сменился бы сообщением о записи настроек.
    $shot = Get-PithShot $window
    Save-PithShot $shot (Resolve-PithOut $Out)
    $shot.Dispose()
    Write-Host "снимок: $Out"

    if ($Check) {
        Click-PithAt ($window.Left + 82) ($window.Top + 479)
        Start-Sleep -Milliseconds 800
    }

    $titles = @(Get-Process -Id $proc.Id | ForEach-Object { $_.MainWindowTitle })
    Write-Host "окна процесса: $($titles -join ', ')"
}
finally {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500

    $settings = Get-PithSandboxSettings $box
    if ($settings) {
        $notion = $settings.notion
        Write-Host ("настройки: токен " + $(if ($notion.token) { "есть" } else { "нет" }) +
            ", рабочая " + $(if ($notion.work_page) { "есть" } else { "нет" }) +
            ", образец " + $(if ($notion.template_page) { "есть" } else { "нет" }))
    }

    if (-not $MenuOnly) { Remove-PithSandbox $box }
}
