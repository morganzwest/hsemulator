; --------------------------------------------------
; HSEmulate Windows Installer (Per-user PATH)
; No admin required
; --------------------------------------------------

[Setup]
AppName=HSEmulate
AppVersion=0.3.0
DefaultDirName={localappdata}\HSEmulate
DisableProgramGroupPage=yes
OutputDir=dist
OutputBaseFilename=hsemulate-0.3.0-windows-x64
Compression=lzma
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest

[Files]
Source: "target\release\hsemulate.exe"; DestDir: "{app}"; Flags: ignoreversion

[Tasks]
Name: addtopath; Description: Add hsemulate to PATH

[Registry]
; Add to USER PATH (no admin)
Root: HKCU; Subkey: "Environment"; \
ValueType: expandsz; ValueName: "Path"; \
ValueData: "{olddata};{app}"; \
Tasks: addtopath; \
Check: NeedsAddPath

[Code]
function NeedsAddPath(): Boolean;
var
  Path: string;
begin
  if not RegQueryStringValue(
    HKCU,
    'Environment',
    'Path',
    Path
  ) then
    Result := True
  else
    Result := Pos(ExpandConstant('{app}'), Path) = 0;
end;
