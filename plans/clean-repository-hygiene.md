# План: Шаг 2 — Архивация исторического балласта и очистка docs/
Ветка: `codex/clean-repository-hygiene`
Цель: Удалить из активного дерева 1 088 устаревших файлов документации, логов, аудитов, тестерских чек-листов и неуправляемых локфайлов (минус ~310 000 строк балласта).

## Что удаляется:
1. `docs/agent-map/` (718 файлов) — исследовательские промежуточные квитанции, карты синтаксиса и тестовые фикстуры аудита (сохранены в ветке `codex/research-reconciliation` на коммите `090c8a19`).
2. `docs/audit-*`, `docs/reference-shots/`, `docs/showcase/`, `docs/release-evidence-*` (210 файлов) — устаревшие растровые скриншоты PNG и сырые логи.
3. `docs/retest-*.html`, `docs/archive-*.html`, `docs/goal-*.md`, `docs/release-notes-*.md`, `docs/PHASE-*.md`, `docs/MIGRATION-*.md` (148 файлов в корне `docs/`).
4. Неуправляемые локфайлы `experiments/macos-gate0a/Cargo.lock` и `experiments/macos-gate0b/Cargo.lock` (12 932 строки).
5. Устаревшие хуки `.claude/` (10 файлов).

## Что остаётся в docs/ (активное рабочее ядро):
- `docs/AGENTS.md` — документация таксономии и правил работы.
- `docs/architecture.md` — актуальная архитектура pure Rust + Slint.
- `docs/retest-template.html` — чистый шаблон чек-листа для релизов.
- `docs/winbrat-recovery.md` — регламент восстановления воркера Winbrat.
- `docs/read-aloud-status.md` — статус и инварианты TTS/OCR сайдкаров.
- `docs/REVIEW_AGENT_PROMPT.md` — регламент независимого рецензирования.
- `docs/memory-architecture.md` — архитектура долговременной памяти SQLite.
- `docs/slint-design-system-and-safe-redesign-plan.md` — дизайн-система и правила UI.

## Доступ к архиву:
Любой старый файл доступен через:
`git show 090c8a19:docs/<путь>` или через ветку `codex/research-reconciliation`.
