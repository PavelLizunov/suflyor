# План: Пакет исправлений DEF-11, DEF-16, DEF-18..20 (P1)
Ветка: `codex/fix-remaining-p1-defects`
Цель: Полностью закрыть оставшиеся 5 дефектов категории P1, завершив разблокировку шлюза P1:

## Что делаем:
1. **DEF-11 (`overlay-backend/src/config.rs`):**
   - В `preserve_corrupt_config` считывать битый файл, и если он валиден по JSON, затирать секреты перед сохранением `json.broken-*`. Ограничивать ротацию файлов `json.broken-*` (хранить максимум 5 последних).
2. **DEF-16 (`slint-experiment/src/bin/overlay_host/settings_controller.rs`):**
   - В `on_language_selected`, `on_tile_monitor_changed` и `on_stealth_changed` сохранять конфигурацию на диск и применять изменения в UI/рантайм только в случае успешного сохранения на диск.
3. **DEF-18 (`slint-experiment/src/bin/overlay_host/window_lifecycle.rs`):**
   - В `do_reveal` проверять результат `apply_stealth_one`: если включен глобальный stealth и наложение WDA завершилось ошибкой, прерывать перемещение окна на экран и оставлять его запаркованным вне экрана `(-32000, -32000)`, предотвращая утечку содержимого в запись экрана.
4. **DEF-19 (`slint-experiment/src/bin/overlay_host/bar_tray.rs`):**
   - В `apply_overlay_hwnd` при отказе захвата HWND под активным stealth не выводить панель в видимую область принудительно, а логировать ошибку и информировать пользователя через иконку в трее.
5. **DEF-20 (`slint-experiment/src/bin/overlay_host/window_lifecycle.rs`):**
   - В `apply_stealth_one` учитывать результат исключения: если хотя бы одно окно не смогло применить WDA, регистрировать отказ в совокупном статусе или сбрасывать `STEALTH_EFFECTIVE`.

## Как проверяем:
1. `git diff --check` на отсутствие форматировочных ошибок.
2. Прогон сборки и тестов на воркере `windows-worker` по точному SHA.
3. Обновление статусов в `DEFECTS.md` (100% P1 закрыты).
