; ============================================================
;  NitroControl — NSIS Installer Script
;  Generates: NitroControl-Setup-x64.exe
;  - Installs to %ProgramFiles%\NitroControl
;  - Desktop + Start Menu shortcuts
;  - Proper uninstaller registered in Control Panel
;  - Requires Windows 10+ x64
; ============================================================

Unicode True
SetCompressor /SOLID lzma
SetCompressorDictSize 64

; --- Metadata ------------------------------------------------
!define APP_NAME        "NitroControl"
!define APP_VERSION     "1.1.0"
!define APP_PUBLISHER   "NitroControl"
!define APP_URL         "https://github.com/RafaelSitompoel/NitroControl"
!define APP_EXE         "NitroControl.exe"
!define BACKEND_EXE     "nitrocontrol-backend.exe"
!define INSTALL_DIR     "$PROGRAMFILES64\${APP_NAME}"
!define UNINST_KEY      "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
!define MUTEX_NAME      "NitroControlSetupMutex"

; --- MUI2 config ---------------------------------------------
!include "MUI2.nsh"
!include "x64.nsh"
!include "LogicLib.nsh"

!define MUI_ABORTWARNING
!define MUI_ICON          "NitroControl.ico"
!define MUI_UNICON        "NitroControl.ico"
!define MUI_HEADERIMAGE
!define MUI_WELCOMEFINISHPAGE_BITMAP_NOSTRETCH

; Welcome page
!define MUI_WELCOMEPAGE_TITLE    "Welcome to NitroControl ${APP_VERSION}"
!define MUI_WELCOMEPAGE_TEXT     "NitroControl lets you control fan speed, monitor temperatures, and manage performance profiles on your Acer Nitro laptop.$\n$\nSupported models: Nitro V15 (AN515-xx), Nitro 5, Nitro 16, Nitro 17.$\n$\nClick Next to continue."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE    "LICENSE.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

!define MUI_FINISHPAGE_RUN         "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT    "Launch NitroControl now"
!define MUI_FINISHPAGE_LINK        "View on GitHub"
!define MUI_FINISHPAGE_LINK_LOCATION "${APP_URL}"
!insertmacro MUI_PAGE_FINISH

; Uninstall pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; --- Installer metadata --------------------------------------
Name              "${APP_NAME} ${APP_VERSION}"
OutFile           "NitroControl-Setup-x64.exe"
InstallDir        "${INSTALL_DIR}"
InstallDirRegKey  HKLM "${UNINST_KEY}" "InstallLocation"
RequestExecutionLevel admin
ShowInstDetails   show
ShowUninstDetails show

; --- Check only one installer instance runs ------------------
Function .onInit
  ${If} ${RunningX64}
    SetRegView 64
  ${Else}
    MessageBox MB_ICONSTOP "NitroControl requires a 64-bit version of Windows."
    Abort
  ${EndIf}

  ; Kill running NitroControl before install
  nsExec::ExecToLog 'taskkill /F /IM "${APP_EXE}" /T'
  nsExec::ExecToLog 'taskkill /F /IM "${BACKEND_EXE}" /T'
  Sleep 800

  ; Kill anything on port 7878 (backend)
  nsExec::ExecToLog 'powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 7878 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"'
  Sleep 400
FunctionEnd

; --- Main install section ------------------------------------
Section "NitroControl (required)" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"

  ; --- Core files ---
  File "NitroControl.exe"
  File "NitroControl.ico"

  ; --- Backend ---
  File "${BACKEND_EXE}"

  ; --- Frontend (web UI served by backend) ---
  SetOutPath "$INSTDIR\frontend\dist"
  File /r "frontend\dist\*.*"

  ; --- Restore install dir ---
  SetOutPath "$INSTDIR"

  ; --- Write uninstall registry ---
  WriteRegStr   HKLM "${UNINST_KEY}" "DisplayName"          "${APP_NAME}"
  WriteRegStr   HKLM "${UNINST_KEY}" "DisplayVersion"        "${APP_VERSION}"
  WriteRegStr   HKLM "${UNINST_KEY}" "Publisher"             "${APP_PUBLISHER}"
  WriteRegStr   HKLM "${UNINST_KEY}" "URLInfoAbout"          "${APP_URL}"
  WriteRegStr   HKLM "${UNINST_KEY}" "InstallLocation"       "$INSTDIR"
  WriteRegStr   HKLM "${UNINST_KEY}" "UninstallString"       '"$INSTDIR\Uninstall.exe"'
  WriteRegStr   HKLM "${UNINST_KEY}" "QuietUninstallString"  '"$INSTDIR\Uninstall.exe" /S'
  WriteRegStr   HKLM "${UNINST_KEY}" "DisplayIcon"           "$INSTDIR\NitroControl.ico"
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify"              1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair"              1
  ; Approximate install size in KB
  WriteRegDWORD HKLM "${UNINST_KEY}" "EstimatedSize"         15000

  ; --- Write uninstaller binary ---
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  ; --- Desktop shortcut ---
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" \
    "$INSTDIR\${APP_EXE}" "" \
    "$INSTDIR\NitroControl.ico" 0 \
    SW_SHOWNORMAL "" "Acer Nitro fan & performance controller"

  ; --- Start Menu shortcut ---
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" \
    "$INSTDIR\${APP_EXE}" "" \
    "$INSTDIR\NitroControl.ico" 0 \
    SW_SHOWNORMAL "" "Acer Nitro fan & performance controller"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" \
    "$INSTDIR\Uninstall.exe" "" \
    "$INSTDIR\Uninstall.exe" 0

SectionEnd

; --- Uninstaller section -------------------------------------
Section "Uninstall"
  ; Stop running processes
  nsExec::ExecToLog 'taskkill /F /IM "${APP_EXE}" /T'
  nsExec::ExecToLog 'taskkill /F /IM "${BACKEND_EXE}" /T'
  Sleep 800

  ; Kill backend on port 7878
  nsExec::ExecToLog 'powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 7878 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"'
  Sleep 400

  ; Remove files
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\${BACKEND_EXE}"
  Delete "$INSTDIR\NitroControl.ico"
  Delete "$INSTDIR\Uninstall.exe"

  ; Remove frontend
  RMDir /r "$INSTDIR\frontend"

  ; Remove user data cache (browser profile) — optional
  ; RMDir /r "$LOCALAPPDATA\NitroControl"

  ; Remove app dir if empty
  RMDir "$INSTDIR"

  ; Remove shortcuts
  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
  RMDir  "$SMPROGRAMS\${APP_NAME}"

  ; Remove registry keys
  DeleteRegKey HKLM "${UNINST_KEY}"

  MessageBox MB_ICONINFORMATION "${APP_NAME} has been uninstalled.$\nYour settings and history were kept at:$\n$LOCALAPPDATA\NitroControl" IDOK +1

SectionEnd
