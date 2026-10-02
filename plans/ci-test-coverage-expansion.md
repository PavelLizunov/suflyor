# План: Шаг 3 — Включение полного набора крейтов в CI и нативный гейт
Ветка: `codex/ci-full-test-coverage`
Цель: Устранить дефекты DEF-21, DEF-22, DEF-25: включить все 5 крейтов (`overlay-backend`, `slint-experiment`, `suflyor-tts`, `suflyor-teratts`, `suflyor-wsola`) в тестовую матрицу Windows в `.github/workflows/ci.yml`, а также актуализировать классификатор `scripts/git-gate-native.ps1`.

## Что делаем:
1. В `.github/workflows/ci.yml`:
   - Добавить шаги проверки форматирования (`fmt --check`), линтера (`clippy -D warnings`) и тестов (`cargo test`) для `suflyor-teratts` и `suflyor-wsola`.
   - Добавить кэширование воркспейсов `suflyor-teratts` и `suflyor-wsola` в `Swatinem/rust-cache@v2`.
2. В `scripts/git-gate-native.ps1`:
   - Исключить файлы `overlay-backend/knowledge/*.md` из docs-only классификатора (так как они компилируются в бинарник через `include_str!`).
   - Добавить сайдкар `suflyor-mlx` в список проверки изменений.

## Что не покрыто:
- Компиляция `suflyor-mlx` требует macOS с установленным Xcode (проверяется отдельным заданием `macos` в CI).

## Как проверяем:
1. `git diff --check` на отсутствие форматировочного дрейфа.
2. Проверка синтаксиса PowerShell через `[System.Management.Automation.Language.Parser]`.
3. Запуск классификатора на `windows-worker` по точному SHA.
4. Обновление `DEFECTS.md` (перевод DEF-21, DEF-22, DEF-25 в статус `Исправлено`).
