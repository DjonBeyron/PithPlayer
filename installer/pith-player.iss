; Установщик Pith Player для Windows (Inno Setup 6).
;
; Собирается скриптом scripts\build_installer.ps1 — он подставляет версию
; и пути. Вручную: ISCC.exe /DVersion=5.1.17 installer\pith-player.iss
;
; Плеер и libmpv лежат в самом установщике. FFmpeg — отдельная история:
; он нужен только для нарезки отрезков, весит больше самого плеера
; и обновляется своим чередом. Поэтому мастер предлагает скачать его
; при установке, а не тащит внутри.
;
; Ключ /DBundledFfmpeg=1 включает старое поведение: FFmpeg кладётся
; в установщик, и загрузка не предлагается вовсе.

#ifndef Version
  #define Version "0.0.0"
#endif

#ifndef Staging
  #define Staging "..\dist\installer-staging"
#endif

#define AppName "Pith Player"
#define AppExe "pith-player.exe"
#define Publisher "Pith"

; Сборка FFmpeg, рекомендованная на ffmpeg.org для Windows. Ссылка
; постоянная и всегда ведёт на последний выпуск; рядом лежит файл
; с контрольной суммой — по нему загруженный архив и проверяется.
#define FfmpegUrl "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
#define FfmpegHashUrl "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip.sha256"

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
; Загруженный FFmpeg приходит в zip, а не в 7z.
ArchiveExtraction=full
; 64-разрядная программа: 32-разрядной сборки нет.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Без прав администратора ставится в профиль пользователя.
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "ru"; MessagesFile: "compiler:Languages\Russian.isl"

[CustomMessages]
en.FfmpegTitle=Additional components
en.FfmpegSubtitle=What else the player needs
en.FfmpegDescription=Cutting clips is done by FFmpeg. The player can download it now — about 90 MB, from the build recommended on ffmpeg.org. Playback works without it; only cutting will be unavailable.
en.FfmpegCheck=Download FFmpeg (recommended)
en.FfmpegFailed=Could not download FFmpeg: %1%nThe player will be installed anyway — cutting will be unavailable until you put ffmpeg.exe and ffprobe.exe next to it.
ru.FfmpegTitle=Дополнительные составляющие
ru.FfmpegSubtitle=Что плееру нужно кроме него самого
ru.FfmpegDescription=Отрезки режет FFmpeg. Плеер может скачать его сейчас — около 90 МБ, сборка, рекомендованная на ffmpeg.org. Без него плеер работает, недоступна только нарезка.
ru.FfmpegCheck=Скачать FFmpeg (рекомендуется)
ru.FfmpegFailed=Не удалось скачать FFmpeg: %1%nПлеер всё равно будет установлен — нарезка станет доступна, когда рядом с ним появятся ffmpeg.exe и ffprobe.exe.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Files]
Source: "{#Staging}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs
#ifndef BundledFfmpeg
; Загруженный архив распаковывается во временную папку: внутри он лежит
; в своей подпапке, а нам нужны только два файла — их разложит код ниже.
Source: "{tmp}\ffmpeg.zip"; DestDir: "{tmp}\ffmpeg"; \
  Flags: external extractarchive recursesubdirs ignoreversion skipifsourcedoesntexist; \
  Check: WantsFfmpeg
#endif

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
; Язык мастера передаётся первому запуску: пользователь уже выбрал его
; здесь, и спрашивать второй раз незачем. Дальше выбор живёт в настройках.
Filename: "{app}\{#AppExe}"; Parameters: "--language={language}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Логи плеера рядом с программой — данные пользователя в профиле не трогаем.
Type: files; Name: "{app}\*.log"
; Скачанный FFmpeg установщик не записывал в свой список — убираем сами.
Type: files; Name: "{app}\ffmpeg.exe"
Type: files; Name: "{app}\ffprobe.exe"

[Code]
#ifndef BundledFfmpeg
var
  FfmpegPage: TInputOptionWizardPage;
  DownloadPage: TDownloadWizardPage;
  FfmpegReady: Boolean;

{ Нужно ли скачивать FFmpeg.
  При тихой установке страницы мастера не показываются, но галочка на ней
  всё равно создана и стоит — тихая установка тоже получает FFmpeg.
  Отказаться можно ключом /NOFFMPEG. }
function WantsFfmpeg: Boolean;
var
  Index: Integer;
begin
  Result := (FfmpegPage <> nil) and FfmpegPage.Values[0];

  if not Result then
    Exit;

  for Index := 1 to ParamCount do
    if CompareText(ParamStr(Index), '/NOFFMPEG') = 0 then begin
      Result := False;
      Exit;
    end;
end;

procedure InitializeWizard;
begin
  FfmpegPage := CreateInputOptionPage(wpSelectTasks,
    ExpandConstant('{cm:FfmpegTitle}'),
    ExpandConstant('{cm:FfmpegSubtitle}'),
    ExpandConstant('{cm:FfmpegDescription}'),
    False, False);
  FfmpegPage.Add(ExpandConstant('{cm:FfmpegCheck}'));
  FfmpegPage.Values[0] := True;

  DownloadPage := CreateDownloadPage(
    SetupMessage(msgWizardPreparing), SetupMessage(msgPreparingDesc), nil);
  DownloadPage.ShowBaseNameInsteadOfUrl := True;
end;

{ Контрольная сумма из файла рядом с архивом. }
function ReadExpectedHash(const FileName: String): String;
var
  Contents: AnsiString;
  Space: Integer;
begin
  Result := '';

  if not LoadStringFromFile(FileName, Contents) then
    Exit;

  Result := Trim(String(Contents));

  { В файле может быть «хеш *имя» — берём только хеш. }
  Space := Pos(' ', Result);
  if Space > 0 then
    Result := Copy(Result, 1, Space - 1);
end;

{ Загрузка идёт здесь, а не по кнопке «Далее»: `PrepareToInstall`
  вызывается и при тихой установке, где кнопок нет вовсе.

  Возвращённая строка означала бы отказ от установки — мы её не возвращаем
  никогда: плеер без FFmpeg работает, недоступна только нарезка. }
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Hash: String;
begin
  Result := '';

  if not WantsFfmpeg or FfmpegReady then
    Exit;

  DownloadPage.Clear;
  DownloadPage.Add('{#FfmpegHashUrl}', 'ffmpeg.sha256', '');
  DownloadPage.Show;

  try
    try
      { Сначала контрольная сумма, потом сам архив — с проверкой по ней.
        Так подменённый или недокачанный файл до диска не доедет. }
      DownloadPage.Download;
      Hash := ReadExpectedHash(ExpandConstant('{tmp}\ffmpeg.sha256'));

      DownloadPage.Clear;
      DownloadPage.Add('{#FfmpegUrl}', 'ffmpeg.zip', Hash);
      DownloadPage.Download;

      FfmpegReady := True;
    except
      { Неудача с загрузкой не отменяет установку: плеер без FFmpeg
        работает, недоступна только нарезка. }
      SuppressibleMsgBox(FmtMessage(ExpandConstant('{cm:FfmpegFailed}'), [GetExceptionMessage]),
        mbInformation, MB_OK, IDOK);
      FfmpegPage.Values[0] := False;
    end;
  finally
    DownloadPage.Hide;
  end;
end;

{ Ищет файл в распакованном дереве: внутри архива он лежит в подпапке. }
function FindTool(const Root, Name: String; var Found: String): Boolean;
var
  Search: TFindRec;
begin
  Result := False;

  if not FindFirst(AddBackslash(Root) + '*', Search) then
    Exit;

  try
    repeat
      if (Search.Name = '.') or (Search.Name = '..') then
        Continue;

      if Search.Attributes and FILE_ATTRIBUTE_DIRECTORY <> 0 then begin
        Result := FindTool(AddBackslash(Root) + Search.Name, Name, Found);
        if Result then
          Exit;
      end else if CompareText(Search.Name, Name) = 0 then begin
        Found := AddBackslash(Root) + Search.Name;
        Result := True;
        Exit;
      end;
    until not FindNext(Search);
  finally
    FindClose(Search);
  end;
end;

{ Кладёт один инструмент рядом с плеером. }
procedure InstallTool(const Root, Name: String);
var
  Found: String;
begin
  if not FindTool(Root, Name, Found) then begin
    Log('в загруженном архиве нет ' + Name);
    Exit;
  end;

  if not FileCopy(Found, ExpandConstant('{app}\') + Name, False) then
    Log('не удалось положить ' + Name + ' рядом с плеером');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Root: String;
begin
  if (CurStep <> ssPostInstall) or not FfmpegReady then
    Exit;

  { Плееру нужны два файла из архива, остальное — документация и ffplay. }
  Root := ExpandConstant('{tmp}\ffmpeg');
  InstallTool(Root, 'ffmpeg.exe');
  InstallTool(Root, 'ffprobe.exe');
  DelTree(Root, True, True, True);
end;
#endif
