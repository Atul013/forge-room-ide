#ifndef PackageDir
  #error PackageDir must point to the prepared package directory
#endif
#ifndef OutputDir
  #error OutputDir must point to the artifact output directory
#endif
#ifndef AppVersion
  #define AppVersion "0.109.0-alpha.1"
#endif

[Setup]
AppId={{AFA3E0AF-AD68-47D2-8593-8B17A4A3F2CE}
AppName=Forge Room Early Preview
AppVersion={#AppVersion}
AppVerName=Forge Room Early Preview {#AppVersion}
AppPublisher=Forge Room contributors
AppPublisherURL=https://github.com/Atul013/forge-room-ide
AppSupportURL=https://github.com/Atul013/forge-room-ide/issues
DefaultDirName={localappdata}\Programs\Forge Room Early Preview
DefaultGroupName=Forge Room Early Preview
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.22000
OutputDir={#OutputDir}
OutputBaseFilename=Forge-Room-Early-Preview-{#AppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
WizardSizePercent=100
WizardImageFile=installer-side.png
WizardSmallImageFile=installer-small.png
SetupIconFile=installer.ico
LicenseFile={#PackageDir}\LICENSE-MIT.txt
InfoBeforeFile={#PackageDir}\EARLY_PREVIEW_NOTES.md
UninstallDisplayName=Forge Room Early Preview
UninstallDisplayIcon={app}\forge-room-early-preview.exe
CloseApplications=yes
RestartApplications=no
SetupLogging=yes
VersionInfoVersion=0.109.0.1
VersionInfoCompany=Forge Room contributors
VersionInfoDescription=Forge Room Early Preview installer
VersionInfoProductName=Forge Room Early Preview
VersionInfoProductVersion=0.109.0.1

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Optional shortcuts:"; Flags: unchecked

[Files]
Source: "{#PackageDir}\forge-room-early-preview.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\EARLY_PREVIEW_NOTES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\LICENSE-MIT.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\THIRD_PARTY_NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\THIRD_PARTY_LICENSES.html"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Forge Room Early Preview"; Filename: "{app}\forge-room-early-preview.exe"; WorkingDir: "{userdocs}"
Name: "{autodesktop}\Forge Room Early Preview"; Filename: "{app}\forge-room-early-preview.exe"; WorkingDir: "{userdocs}"; Tasks: desktopicon

[Run]
Filename: "{app}\forge-room-early-preview.exe"; Description: "Launch Forge Room Early Preview"; Flags: nowait postinstall skipifsilent
