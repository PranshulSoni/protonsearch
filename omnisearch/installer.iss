[Setup]
AppName=omnisearch
AppVersion=1.0.3
DefaultDirName={localappdata}\Programs\omnisearch
DefaultGroupName=omnisearch
UninstallDisplayIcon={app}\omnisearch.exe
SetupIconFile=..\icons\OmniSearchTrans.ico
Compression=lzma2
SolidCompression=yes
OutputDir=setup
OutputBaseFilename=omnisearchsetup
PrivilegesRequired=lowest
CloseApplications=yes
RestartApplications=no

[Files]
Source: "target\release\omnisearch.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\uninstall.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\omnisearch"; Filename: "{app}\omnisearch.exe"
Name: "{userdesktop}\omnisearch"; Filename: "{app}\omnisearch.exe"
Name: "{group}\Uninstall omnisearch"; Filename: "{app}\uninstall.exe"

[Run]
Filename: "{app}\omnisearch.exe"; Description: "Launch omnisearch"; Flags: nowait postinstall

[UninstallRun]
Filename: "taskkill"; Parameters: "/F /IM omnisearch.exe"; Flags: runhidden; RunOnceId: "KillApp"
Filename: "taskkill"; Parameters: "/F /IM hermes.exe"; Flags: runhidden; RunOnceId: "KillHermes"

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\omnisearch"

[Code]
// Guarantee the running app is closed right before file replacement. CloseApplications=yes
// (Windows Restart Manager) is a best-effort graceful close first, but a hidden tray app may
// not respond to it, so we force it here. This code runs INSIDE the installer process
// (omnisearchsetup.exe), never omnisearch.exe — so it can never kill itself or this installer.
procedure TerminateApp;
var
  ResultCode: Integer;
  Retries: Integer;
begin
  Exec('taskkill.exe', '/F /IM omnisearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM hermes.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  
  // Retry loop: wait up to 5 seconds for the exe file handle to be released.
  // Re-attempt taskkill halfway through in case a process respawned.
  Retries := 0;
  while Retries < 10 do
  begin
    Sleep(500);
    if not FileExists(ExpandConstant('{app}\omnisearch.exe')) then
      Break;
    if Retries = 5 then
      Exec('taskkill.exe', '/F /IM omnisearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Retries := Retries + 1;
  end;
end;

// PrepareToInstall runs just before the file copy, so the exe is guaranteed free by then.
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  NeedsRestart := False;
  TerminateApp;
  Result := '';
end;
