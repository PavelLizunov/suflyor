# План: Формирование релиз-кандидата v0.38.1-rc.5 (Шаг 7)
Ветка: `codex/release-v0.38.1-rc5`
Версия: `0.38.1-rc.5`
Цель: Сборка и валидация стабильного плавающего релизного кандидата v0.38.1-rc.5 после закрытия 100% P0/P1 дефектов и очистки 1 088 файлов балласта.

## Что сделано:
1. Синхронизирована версия `0.38.1-rc.5` в:
   - `slint-experiment/Cargo.toml`
   - `scripts/slint-installer.nsi`
   - `slint-experiment/macos/Info.plist`
   - `CURRENT_STATE.md`
2. Обновлен издатель в NSIS инсталляторе: `PRODUCT_PUBLISHER "suflyor"`.

## Как проверяем:
1. Запуск теста `version_guard` на `windows-worker`.
2. Запуск сборщика `powershell -ExecutionPolicy Bypass -File scripts\build-slint-release.ps1 -Installer` на `windows-worker`.
3. Валидация наличия и целостности артефактов:
   - `overlay-host.exe`
   - `suflyor-tts.exe`
   - `suflyor-teratts.exe`
   - `DirectML.dll`
   - `bundle/suflyor-slint-setup.exe`
4. Расчет хэшей SHA-256 и размеров файлов.
