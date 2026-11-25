; Installeur Windows de Planif'Week (Inno Setup 6).
; Compiler avec installer\build-installer.ps1, qui passe la version de Cargo.toml.

#define AppName "Planif'Week"
#define AppExe "planif-week.exe"
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
; Identifiant fixe : ne pas le changer, il permet aux nouvelles versions de remplacer l'ancienne.
AppId={{7C3E5B1A-4F2D-4C8E-9A61-2D5F0B8E4A17}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppName}
DefaultDirName={autopf}\PlanifWeek
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
; Installation pour l'utilisateur courant, sans droits administrateur (au choix dans l'assistant).
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\target\installer
OutputBaseFilename=PlanifWeek-Setup-{#AppVersion}
SetupIconFile=..\assets\planif-week.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
VersionInfoVersion={#AppVersion}
VersionInfoProductName={#AppName}
VersionInfoDescription=Installation de {#AppName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; Rien n'est créé à l'installation : l'application gère elle-même le lancement au démarrage
; (Paramètres). La désinstallation retire l'entrée si elle existe.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "PlanifWeek"; Flags: uninsdeletevalue dontcreatekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "PlanifWeek"; Flags: uninsdeletevalue dontcreatekey

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

; Les tâches de l'utilisateur (%APPDATA%\PlanifWeek) ne sont volontairement pas supprimées
; à la désinstallation : une réinstallation les retrouve.
