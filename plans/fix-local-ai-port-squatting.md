# План: Устранение уязвимости port-squatting в is_reachable (DEF-05, P0)
Ветка: `codex/fix-local-ai-port-squatting`
Цель: Устранить дефект DEF-05 в `overlay-backend/src/local_ai.rs`, где `is_reachable` вызывал `curl` без флага `-f` (`--fail`), принимая любые коды ответов (HTTP 404, 500, 503 или ответы сторонних сервисов) за рабочий сервер ИИ и пропуская запуск управляемого `llama-server`.

## Что делаем:
1. В `overlay-backend/src/local_ai.rs`:
   - В функции `is_reachable` добавить аргумент `"-f"` в параметры вызова `curl_exe()`, чтобы запросы с кодами HTTP >= 400 завершались с ошибкой (код 22) и возвращали `false`.
2. В `overlay-backend/src/local_ai/tests.rs`:
   - Написать характеризационный тест `is_reachable_requires_http_success_and_rejects_http_errors`, подтверждающий, что `is_reachable` возвращает `true` на HTTP 200 и `false` на HTTP 404/503.

## Как проверяем:
1. `git diff --check` на отсутствие форматировочных ошибок.
2. Прогон unit-теста `is_reachable_requires_http_success_and_rejects_http_errors` на `windows-worker`.
3. Обновление статуса DEF-05 в `DEFECTS.md` (все P0 закрыты).
