[Setup]
AppName=OpenSearch OS
AppVersion=0.1.0
DefaultDirName={localappdata}\Programs\OpenSearch OS
DefaultGroupName=OpenSearch OS
UninstallDisplayIcon={app}\opensearch-os.exe
Compression=lzma2
SolidCompression=yes
OutputDir=setup
OutputBaseFilename=OpenSearchOSSetup
PrivilegesRequired=lowest

[Files]
Source: "target\release\opensearch-os.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\uninstall.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\OpenSearch OS"; Filename: "{app}\opensearch-os.exe"
Name: "{userstartup}\OpenSearch OS"; Filename: "{app}\opensearch-os.exe"
Name: "{userdesktop}\OpenSearch OS"; Filename: "{app}\opensearch-os.exe"
Name: "{group}\Uninstall OpenSearch OS"; Filename: "{app}\uninstall.exe"


[Run]
Filename: "{app}\opensearch-os.exe"; Description: "Launch OpenSearch OS"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill"; Parameters: "/F /IM opensearch-os.exe"; Flags: runhidden; RunOnceId: "KillApp"
Filename: "taskkill"; Parameters: "/F /IM hermes.exe"; Flags: runhidden; RunOnceId: "KillHermes"

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\opensearch-os"
