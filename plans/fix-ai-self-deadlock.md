# План: Устранение самоблокировки в complete_exclusive (DEF-02, P0)
Ветка: `codex/fix-ai-self-deadlock`
Цель: Устранить критический self-deadlock в `complete_exclusive`, где задача захватывает оба разрешения семафора `AI_SEMAPHORE.acquire_many(2)`, а затем вызываемый внутренний метод `complete_with_usage_inner` повторно пытается запросить третье разрешение, вызывая вечное зависание.

## Что делаем:
1. В `overlay-backend/src/ai/completion.rs`:
   - Реализовать `complete_with_usage_guarded`, принимающую флаг `acquire_semaphore: bool`.
   - В `complete_exclusive` вызывать `complete_with_usage_guarded` с `acquire_semaphore: false`, так как эксклюзивный доступ уже гарантирован вызовом `acquire_exclusive_ai()`.
2. В `overlay-backend/src/ai/tests.rs`:
   - Написать характеризационный асинхронный тест `complete_exclusive_does_not_deadlock_on_semaphore`, подтверждающий отсутствие блокировки по таймауту при вызове `complete_exclusive`.

## Как проверяем:
1. `git diff --check` на отсутствие висячих пробелов.
2. Прогон целевого теста `complete_exclusive_does_not_deadlock_on_semaphore` на `windows-worker`.
3. Обновление статуса DEF-02 в `DEFECTS.md`.
