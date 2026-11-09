[Setup]
AppName=ProtonSearch
AppVersion=1.1.0
DefaultDirName={localappdata}\Programs\ProtonSearch
DefaultGroupName=ProtonSearch
UninstallDisplayIcon={app}\ProtonSearch.exe
SetupIconFile=..\assets\brand\runtime\protonsearch.ico
Compression=lzma2
SolidCompression=yes
OutputDir=setup
OutputBaseFilename=protonsearchsetup
PrivilegesRequired=lowest
CloseApplications=yes
RestartApplications=no

[Files]
Source: "target\release\ProtonSearch.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\uninstall.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\ProtonSearch"; Filename: "{app}\ProtonSearch.exe"
Name: "{userdesktop}\ProtonSearch"; Filename: "{app}\ProtonSearch.exe"
Name: "{group}\Uninstall ProtonSearch"; Filename: "{app}\uninstall.exe"

[Run]
Filename: "{app}\ProtonSearch.exe"; Description: "Launch ProtonSearch"; Flags: nowait postinstall

[UninstallRun]
Filename: "taskkill"; Parameters: "/F /IM ProtonSearch.exe"; Flags: runhidden; RunOnceId: "KillApp"
Filename: "taskkill"; Parameters: "/F /IM hermes.exe"; Flags: runhidden; RunOnceId: "KillHermes"

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\protonsearch"

[Code]
// Guarantee the running app is closed right before file replacement. CloseApplications=yes
// (Windows Restart Manager) is a best-effort graceful close first, but a hidden tray app may
// not respond to it, so we force it here. This code runs INSIDE the installer process
// (protonsearchsetup.exe), never ProtonSearch.exe — so it can never kill itself or this installer.
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
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM ProtonSearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM ProtonSearch.bak', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM protonsearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM protonsearch.bak', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM omnisearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM omnisearch.bak', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM opensearch-os.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM opensearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM hermes.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM hermes.bak', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  
  Sleep(500); // let processes terminate

  // Rename fallback for ProtonSearch.exe
  AppPath := ExpandConstant('{app}\ProtonSearch.exe');
  if FileExists(AppPath) then
  begin
    Renamed := False;
    // Try standard .bak first
    BackupPath := ExpandConstant('{app}\ProtonSearch.bak');
    DeleteFile(BackupPath);
    if RenameFile(AppPath, BackupPath) then
    begin
      Renamed := True;
    end else begin
      // Try unique names .bak1, .bak2 ... if standard .bak is locked
      for I := 1 to 5 do
      begin
        BackupPath := ExpandConstant('{app}\ProtonSearch.bak' + IntToStr(I));
        DeleteFile(BackupPath);
        if RenameFile(AppPath, BackupPath) then
        begin
          Renamed := True;
          Break;
        end;
      end;
    end;

    if Renamed then
      Log('Successfully renamed locked ProtonSearch.exe')
    else
      Log('Failed to rename locked ProtonSearch.exe');
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

procedure RemoveLegacyOmniSearchInstall;
var
  LegacyInstallDir: String;
  LegacyProgramsDir: String;
  ResultCode: Integer;
begin
  // Remove the old OmniSearch application install while keeping %APPDATA%\omnisearch intact.
  // ProtonSearch migrates that data directory on first launch; deleting it here would lose
  // indexes, settings, clipboard history, snippets, agents, and AI config.
  LegacyInstallDir := ExpandConstant('{localappdata}\Programs\omnisearch');
  if DirExists(LegacyInstallDir) then
  begin
    Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM omnisearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    DelTree(LegacyInstallDir, True, True, True);
    Log('Removed legacy OmniSearch install directory: ' + LegacyInstallDir);
  end;

  DeleteFile(ExpandConstant('{userdesktop}\omnisearch.lnk'));
  LegacyProgramsDir := ExpandConstant('{userprograms}\omnisearch');
  if DirExists(LegacyProgramsDir) then
  begin
    DelTree(LegacyProgramsDir, True, True, True);
    Log('Removed legacy OmniSearch Start Menu folder: ' + LegacyProgramsDir);
  end;

  // Also clean up the lowercase protonsearch folder if we are migrating to capitalized ProtonSearch
  LegacyInstallDir := ExpandConstant('{localappdata}\Programs\protonsearch');
  if DirExists(LegacyInstallDir) then
  begin
    Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM protonsearch.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    DelTree(LegacyInstallDir, True, True, True);
    Log('Removed legacy lowercase protonsearch install directory: ' + LegacyInstallDir);
  end;

  DeleteFile(ExpandConstant('{userdesktop}\protonsearch.lnk'));
  LegacyProgramsDir := ExpandConstant('{userprograms}\protonsearch');
  if DirExists(LegacyProgramsDir) then
  begin
    DelTree(LegacyProgramsDir, True, True, True);
    Log('Removed legacy lowercase protonsearch Start Menu folder: ' + LegacyProgramsDir);
  end;

  RegDeleteKeyIncludingSubkeys(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\omnisearch_is1');
  RegDeleteKeyIncludingSubkeys(HKLM, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\omnisearch_is1');
  RegDeleteKeyIncludingSubkeys(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\protonsearch_is1');
  RegDeleteKeyIncludingSubkeys(HKLM, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\protonsearch_is1');
end;

// PrepareToInstall runs just before the file copy, so the exe is guaranteed free by then.
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  NeedsRestart := False;
  TerminateApp;
  RemoveLegacyOmniSearchInstall;
  Result := '';
end;
