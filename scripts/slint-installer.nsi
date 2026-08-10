; NSIS installer for the suflyor overlay-host binary (Slint UI).
;
; Installs into %LOCALAPPDATA%\suflyor-slint\ (per-user, no admin).
; Creates a Start menu shortcut + Add/Remove Programs entry.
; Uninstaller in the install dir.
;
; Build:
;   makensis scripts/slint-installer.nsi
; Output:
;   slint-experiment/target/release/bundle/suflyor-slint-setup.exe

!define PRODUCT_NAME "suflyor"
!define PRODUCT_VERSION "0.37.0-rc.1"
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
  ; Read-aloud (TTS) sidecar — a separate process so its neural-TTS onnxruntime
  ; never shares a binary with the app's ort/GigaAM STT (two static onnxruntimes
  ; crash). overlay-host spawns it from beside itself. The voices themselves are
  ; NOT bundled (too large) — installed on demand from Settings -> "Озвучка".
  File "..\slint-experiment\target\release\suflyor-tts.exe"
  ; RC17 — experimental TeraTTSv2 read-aloud sidecar (ort ONNX graphs). Its
  ; ~370 MB model is NEVER bundled here — downloaded on demand from the pinned
  ; upstream revision via Settings -> Read aloud -> Tera -> Install model.
  ; See suflyor-teratts/NOTICE.md: upstream ships no LICENSE, so redistribution
  ; of the weights is gated until an archived author grant is on file.
  File "..\slint-experiment\target\release\suflyor-teratts.exe"
  ; onnxruntime (GigaAM STT) is STATICALLY linked into the exe (ort 2.0
  ; download-binaries, no load-dynamic) -> no onnxruntime.dll to ship.
  ; The statically linked DirectML provider imports DMLCreateDevice1 at process
  ; startup. Older Windows 10 builds ship a system DirectML.dll without that
  ; export, so bundle the exact redistributable selected by ort at build time.
  File "..\slint-experiment\target\release\DirectML.dll"
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
  Delete "$INSTDIR\suflyor-tts.exe"
  Delete "$INSTDIR\suflyor-teratts.exe"
  Delete "$INSTDIR\DirectML.dll"
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

  ; fs-audit — offer to remove user data + downloaded AI models. Opt-in so a
  ; reinstall/upgrade keeps the user's sessions by default. $APPDATA / $PROFILE
  ; are per-user (RequestExecutionLevel user), so no elevation is needed. Covers
  ; the brand data dir AND the legacy "overlay-mvp" name (in case the build was
  ; uninstalled before ever launching the renamed version), plus the model tree.
  MessageBox MB_YESNO|MB_ICONQUESTION "Удалить также ваши данные (настройки, история сессий, записи) и скачанные модели ИИ?$\n$\nДа — стереть всё с диска. Нет — оставить данные (на случай переустановки)." IDNO uninst_keep_data
    RMDir /r "$APPDATA\suflyor"
    RMDir /r "$APPDATA\overlay-mvp"
    RMDir /r "$PROFILE\suflyor-local-ai"
  uninst_keep_data:

  DeleteRegKey HKCU "${PRODUCT_UNINST_KEY}"
SectionEnd
