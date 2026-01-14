; --------------------------------------------------
; HSEmulate Windows Installer
; Per-user install (no admin)
; Winget compatible
; --------------------------------------------------

[Setup]
AppName=HSEmulate
AppVersion=0.3.5

; Windows metadata (important for winget & Apps & Features)
VersionInfoVersion=0.3.5
VersionInfoProductVersion=0.3.5
VersionInfoProductName=HSEmulate
VersionInfoDescription=HSEmulate CLI

DefaultDirName={localappdata}\HSEmulate
DisableProgramGroupPage=yes

OutputDir=dist
OutputBaseFilename=hsemulate-0.3.5-windows-x64-installer

Compression=lzma
SolidCompression=yes

ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest

; --------------------------------------------------
; Files
; --------------------------------------------------

[Files]
Source: "target\release\hsemulate.exe"; DestDir: "{app}"; Flags: ignoreversion

; --------------------------------------------------
; Tasks
; --------------------------------------------------

[Tasks]
; Must be auto-selected for silent installs (winget)
Name: addtopath; Description: Add hsemulate to PATH; Flags: checkedonce

; --------------------------------------------------
; Registry (USER PATH only — no admin)
; --------------------------------------------------

[Registry]
Root: HKCU; Subkey: "Environment"; \
ValueType: expandsz; ValueName: "Path"; \
ValueData: "{code:GetNewPath}"; \
Tasks: addtopath; \
Check: NeedsAddPath

; --------------------------------------------------
; Code
; --------------------------------------------------

[Code]

function NeedsAddPath(): Boolean;
var
  Path: string;
begin
  if RegQueryStringValue(HKCU, 'Environment', 'Path', Path) then
    Result := Pos(ExpandConstant('{app}'), Path) = 0
  else
    Result := True;
end;

function GetNewPath(Param: string): string;
var
  Path: string;
begin
  if RegQueryStringValue(HKCU, 'Environment', 'Path', Path) then
    Result := Path + ';' + ExpandConstant('{app}')
  else
    Result := ExpandConstant('{app}');
end;
