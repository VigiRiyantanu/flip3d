; Inno Setup Script for Flip 3D (Vista Style)
; Original Windhawk mod by caliberda, ported to standalone Rust by Antigravity

#ifndef MyAppVersion
#define MyAppVersion "1.7.1"
#endif

#define MyAppName "Flip 3D"
#define MyAppPublisher "Antigravity"
#define MyAppURL "https://github.com/cesarkali/alt-tab-flip-3d-fork"
#define MyAppExeName "flip3d.exe"
#define MySettingsExeName "settings-ui.exe"

[Setup]
AppId={{9B7E38B1-02E6-4A59-86BC-95D94D251D44}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\Flip 3D
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=Flip3D-Setup-v{#MyAppVersion}-x64
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\app.ico
MinVersion=10.0.17763
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
CloseApplications=yes
CloseApplicationsFilter=flip3d.exe,settings-ui.exe
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\{#MySettingsExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "MicrosoftEdgeWebview2Setup.exe"; DestDir: "{tmp}"; Flags: deleteafterinstall

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#MyAppExeName}"
Name: "{group}\{#MyAppName} Settings"; Filename: "{app}\{#MySettingsExeName}"; IconFilename: "{app}\{#MySettingsExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon; IconFilename: "{app}\{#MyAppExeName}"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
procedure KillRunningApp();
var
  ResultCode: Integer;
begin
  Exec('taskkill.exe', '/F /IM flip3d.exe /IM settings-ui.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

// Detect if Microsoft Edge WebView2 Runtime is installed
function IsWebView2Installed(): Boolean;
var
  Version: String;
begin
  Result := False;
  // Check 64-bit / 32-bit HKLM
  if RegQueryStringValue(HKLM, 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) then
  begin
    if (Version <> '') and (Version <> '0.0.0.0') then
    begin
      Result := True;
      Exit;
    end;
  end;

  if RegQueryStringValue(HKLM, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) then
  begin
    if (Version <> '') and (Version <> '0.0.0.0') then
    begin
      Result := True;
      Exit;
    end;
  end;

  // Check HKCU
  if RegQueryStringValue(HKCU, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) then
  begin
    if (Version <> '') and (Version <> '0.0.0.0') then
    begin
      Result := True;
      Exit;
    end;
  end;
end;

// Prepare to install: Kill running processes and deploy WebView2 if missing
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
  InstallerPath: String;
begin
  Result := '';
  KillRunningApp();

  if not IsWebView2Installed() then
  begin
    InstallerPath := ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe');
    if FileExists(InstallerPath) then
    begin
      if Exec(InstallerPath, '/silent /install', '', SW_SHOW, ewWaitUntilTerminated, ResultCode) then
      begin
        if not IsWebView2Installed() then
        begin
          MsgBox('Microsoft Edge WebView2 Runtime is required for Flip 3D Settings.' + #13#10 +
                 'The installer attempted to install it silently but could not verify completion.' + #13#10 +
                 'If settings do not open, please install WebView2 from:' + #13#10 +
                 'https://go.microsoft.com/fwlink/p/?LinkId=2124703', mbInformation, MB_OK);
        end;
      end
      else
      begin
        MsgBox('Could not launch the WebView2 installer bootstrapper.' + #13#10 +
               'Please install Microsoft Edge WebView2 Runtime manually from:' + #13#10 +
               'https://go.microsoft.com/fwlink/p/?LinkId=2124703', mbError, MB_OK);
      end;
    end;
  end;
end;

// Uninstallation steps: Cleanup registry autostart and prompt for settings deletion
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppDataPath: String;
  LocalAppDataPath: String;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // Clean up startup registry key if present
    RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Flip3D');

    // Prompt user to delete user data (Default NO)
    if MsgBox('Do you also want to delete your Flip 3D settings and logs?' + #13#10 +
              '(%AppData%\Flip3D and %LocalAppData%\Flip3D)',
              mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
    begin
      AppDataPath := ExpandConstant('{userappdata}\Flip3D');
      LocalAppDataPath := ExpandConstant('{localappdata}\Flip3D');
      DelTree(AppDataPath, True, True, True);
      DelTree(LocalAppDataPath, True, True, True);
    end;
  end;
end;
