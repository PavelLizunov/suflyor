; Phase D3 — NSIS installer for the Slint v0.2.0-pre overlay-host binary.
;
; Installs into %LOCALAPPDATA%\suflyor-slint\ (separate from existing
; suflyor v0.1.1 — both can coexist for side-by-side testing). Creates
; a Start menu shortcut + Add/Remove Programs entry. Uninstaller in
; the install dir.
;
; Build:
;   makensis scripts/slint-installer.nsi
; Output:
;   slint-experiment/target/release/bundle/suflyor-slint-setup.exe
;
; Phase E (full v0.2.0 cut) will rename product back to "suflyor"
; and bump the upgrade GUID so it overwrites v0.1.1 cleanly.

!define PRODUCT_NAME "suflyor"
!define PRODUCT_VERSION "0.2.0"
!define PRODUCT_PUBLISHER "x3d_mutant"
!define PRODUCT_EXE "overlay-host.exe"
!define PRODUCT_INSTALL_DIR "$LOCALAPPDATA\suflyor-slint"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\suflyor-slint"

SetCompressor /SOLID lzma
Unicode true
RequestExecutionLevel user
Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "..\slint-experiment\target\release\bundle\suflyor-slint-setup.exe"
InstallDir "${PRODUCT_INSTALL_DIR}"
ShowInstDetails show
ShowUnInstDetails show

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Main" SEC_MAIN
  SetOutPath "$INSTDIR"
  File "..\slint-experiment\target\release\${PRODUCT_EXE}"
  ; App icon — used by the Start-menu + Desktop shortcuts and the
  ; Add/Remove Programs entry (the .exe also embeds it via build.rs).
  File "..\slint-experiment\assets\icon.ico"
  ; Bundle translations directory so @tr() can find .po files at runtime.
  ; (Slint compiles translations INTO the binary via with_bundled_translations,
  ; so technically not needed at runtime; included for future runtime-load mode.)
  SetOutPath "$INSTDIR\translations\ru\LC_MESSAGES"
  File "..\slint-experiment\translations\ru\LC_MESSAGES\slint-replay.po"

  ; Start menu shortcut
  CreateDirectory "$SMPROGRAMS\suflyor-slint"
  CreateShortcut "$SMPROGRAMS\suflyor-slint\suflyor (Slint).lnk" "$INSTDIR\${PRODUCT_EXE}" "" "$INSTDIR\icon.ico" 0

  ; Desktop shortcut
  CreateShortcut "$DESKTOP\suflyor (Slint).lnk" "$INSTDIR\${PRODUCT_EXE}" "" "$INSTDIR\icon.ico" 0

  ; Uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Add/Remove Programs entry
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "InstallLocation" "$\"$INSTDIR$\""
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$\"$INSTDIR\icon.ico$\""
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "NoModify" "1"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "NoRepair" "1"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${PRODUCT_EXE}"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$INSTDIR\translations\ru\LC_MESSAGES\slint-replay.po"
  RMDir "$INSTDIR\translations\ru\LC_MESSAGES"
  RMDir "$INSTDIR\translations\ru"
  RMDir "$INSTDIR\translations"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\suflyor-slint\suflyor (Slint).lnk"
  RMDir "$SMPROGRAMS\suflyor-slint"
  Delete "$DESKTOP\suflyor (Slint).lnk"
  DeleteRegKey HKCU "${PRODUCT_UNINST_KEY}"
SectionEnd
