[Setup]
AppName=omnisearch
AppVersion=1.0.4
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
  I: Integer;
  Renamed: Boolean;
begin
  // Force kill all possible process names using full path to taskkill
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM omnisearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM omnisearch.bak', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM opensearch-os.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM opensearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM hermes.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM hermes.bak', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  
  Sleep(500); // let processes terminate

  // Rename fallback for omnisearch.exe
  AppPath := ExpandConstant('{app}\omnisearch.exe');
  if FileExists(AppPath) then
  begin
    Renamed := False;
    // Try standard .bak first
    BackupPath := ExpandConstant('{app}\omnisearch.bak');
    DeleteFile(BackupPath);
    if RenameFile(AppPath, BackupPath) then
    begin
      Renamed := True;
    end else begin
      // Try unique names .bak1, .bak2 ... if standard .bak is locked
      for I := 1 to 5 do
      begin
        BackupPath := ExpandConstant('{app}\omnisearch.bak' + IntToStr(I));
        DeleteFile(BackupPath);
        if RenameFile(AppPath, BackupPath) then
        begin
          Renamed := True;
          Break;
        end;
      end;
    end;
    
    if Renamed then
      Log('Successfully renamed locked omnisearch.exe')
    else
      Log('Failed to rename locked omnisearch.exe');
  end;

  // Rename fallback for hermes.exe
  HermesPath := ExpandConstant('{app}\hermes.exe');
  if FileExists(HermesPath) then
  begin
    Renamed := False;
    HermesBackupPath := ExpandConstant('{app}\hermes.bak');
    DeleteFile(HermesBackupPath);
    if RenameFile(HermesPath, HermesBackupPath) then
    begin
      Renamed := True;
    end else begin
      for I := 1 to 5 do
      begin
        HermesBackupPath := ExpandConstant('{app}\hermes.bak' + IntToStr(I));
        DeleteFile(HermesBackupPath);
        if RenameFile(HermesPath, HermesBackupPath) then
        begin
          Renamed := True;
          Break;
        end;
      end;
    end;
  end;
end;

// PrepareToInstall runs just before the file copy, so the exe is guaranteed free by then.
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  NeedsRestart := False;
  TerminateApp;
  Result := '';
end;
