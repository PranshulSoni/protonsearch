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
  AppPath: String;
  BackupPath: String;
  HermesPath: String;
  HermesBackupPath: String;
begin
  // Force kill processes using full path to taskkill
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM omnisearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM hermes.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  
  Sleep(500); // let processes terminate

  // Rename fallback for omnisearch.exe
  AppPath := ExpandConstant('{app}\omnisearch.exe');
  BackupPath := ExpandConstant('{app}\omnisearch.bak');
  if FileExists(AppPath) then
  begin
    // Attempt to delete any old backup first
    DeleteFile(BackupPath);
    // Rename the locked file to .bak so the installer can write a new omnisearch.exe
    if RenameFile(AppPath, BackupPath) then
    begin
      Log('Successfully renamed locked omnisearch.exe to omnisearch.bak');
    end else begin
      Log('Failed to rename locked omnisearch.exe');
    end;
  end;

  // Rename fallback for hermes.exe
  HermesPath := ExpandConstant('{app}\hermes.exe');
  HermesBackupPath := ExpandConstant('{app}\hermes.bak');
  if FileExists(HermesPath) then
  begin
    DeleteFile(HermesBackupPath);
    RenameFile(HermesPath, HermesBackupPath);
  end;
end;

// PrepareToInstall runs just before the file copy, so the exe is guaranteed free by then.
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  NeedsRestart := False;
  TerminateApp;
  Result := '';
end;
