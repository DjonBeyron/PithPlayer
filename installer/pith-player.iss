; Установщик Pith Player для Windows (Inno Setup 6).
;
; Собирается скриптом scripts\build_installer.ps1 — он подставляет версию
; и пути. Вручную: ISCC.exe /DVersion=5.0.10 installer\pith-player.iss
;
; Установщик кладёт плеер, libmpv и — если они нашлись при сборке —
; ffmpeg с ffprobe. Без FFmpeg плеер работает, недоступна только нарезка.

#ifndef Version
  #define Version "0.0.0"
#endif

#ifndef Staging
  #define Staging "..\dist\installer-staging"
#endif

#define AppName "Pith Player"
#define AppExe "pith-player.exe"
#define Publisher "Pith"

[Setup]
; Единожды выданный номер: по нему Windows узнаёт установленную программу
; и обновляет её на месте, а не ставит второй копией.
AppId={{8E5C1B0A-4F2D-4C58-9E3B-1B7A9D6C3F41}
AppName={#AppName}
AppVersion={#Version}
AppPublisher={#Publisher}
DefaultDirName={autopf}\PithPlayer
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=PithPlayer-{#Version}-setup
SetupIconFile=..\crates\pith-app\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; 64-разрядная программа: 32-разрядной сборки нет.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Без прав администратора ставится в профиль пользователя.
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "ru"; MessagesFile: "compiler:Languages\Russian.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Files]
Source: "{#Staging}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Логи плеера рядом с программой — данные пользователя в профиле не трогаем.
Type: files; Name: "{app}\*.log"
