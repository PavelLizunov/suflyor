# План: Пакет исправлений приватности и защиты секретов (DEF-06..DEF-11, P1)
Ветка: `codex/fix-privacy-and-backups`
Цель: Устранить 5 дефектов безопасности и утечек данных (DEF-06, DEF-07, DEF-08, DEF-09, DEF-10):

## Что делаем:
1. **DEF-06 (`slint-experiment/src/bin/overlay_host/diagnostics.rs`):**
   - В `redact_secrets` исправить пропуск токена при двойном пробеле после `Bearer ` (пропуск ведущих пробелов перед вычислением длины токена).
2. **DEF-07 (`slint-experiment/src/bin/overlay_host/diagnostics.rs`):**
   - В `redact_urls` добавить схемы `ws://`, `wss://`, `ftp://`, `file://` в список поиска префиксов URL для маскирования хостов.
3. **DEF-08 (`overlay-backend/src/config.rs`):**
   - В `mask_host` отсекать параметры строки запроса (`?`) и фрагменты (`#`) из пути URL, чтобы ключи API и токены в строке запроса не попадали в отчёты диагностики.
4. **DEF-09 (`overlay-backend/src/config.rs`):**
   - В методе `readiness` применять `mask_host` к `ep.base_url` и `self.stt_whisper_url`.
5. **DEF-10 (`overlay-backend/src/config.rs`):**
   - В `save_to_path` при создании файла резервной копии `config.json.bak` на Unix задавать права доступа `mode(0o600)` (чтение/запись только владельцем).

## Как проверяем:
1. `git diff --check` на отсутствие форматировочных ошибок.
2. Unit-тесты для санитизации double-space `Bearer `, схем WebSocket и query-параметров.
3. Прогон тестов на воркере `windows-worker` по точному SHA.
4. Обновление статусов в `DEFECTS.md`.
