# Kickoff prompt — Slint migration

**Use:** paste the block below into a fresh Claude Code session in this
project. Future Claude will read memory + CLAUDE.md + the migration
plan and start executing Phase 0 immediately.

**Why a kickoff doc:** session continuity. The new session does NOT
have this chat's context — it has only memory files + CLAUDE.md +
docs/. This prompt tells it where to look and what tempo to keep.

---

## Copy this verbatim into the new session

```
Старт Slint migration для overlay-mvp.

ПЕРЕД ВСЕМ — прочти в этом порядке:
1. CLAUDE.md (auto-injected)
2. memory/MEMORY.md (auto-injected)
3. memory/reference_vpnctl_methodology.md
4. memory/reference_vpnctl_setup_addendum.md
5. memory/reference_suflyor_gui_strictness_spec.md
6. memory/reference_slint_ecosystem.md
7. memory/feedback_no_marathon_releases.md
8. memory/project_overlay_mvp_history.md
9. memory/user_setup_monitors.md
10. docs/MIGRATION-PLAN-SLINT.md
11. docs/ADR-001-stack.md
12. docs/REVIEW_AGENT_PROMPT.md
13. docs/KICKOFF-SLINT-MIGRATION.md (этот файл)

КОНТЕКСТ: предыдущая сессия 2026-05-27 завершила Tier 1-5 harness на
React+Tauri (коммиты e33d69e..b851d85 на master). Решение пользователя —
мигрировать на Slint. План в docs/MIGRATION-PLAN-SLINT.md.

ПОЛЬЗОВАТЕЛЬ ХОЧЕТ:
- Выполнить ВСЕ 8 фаз плана (0-7) agent-fast темпом — БЕЗ человеко-
  недельных задержек между фазами. Думай в часах/днях агент-времени,
  не в неделях. План указывает 12 недель solo human pace; реальная
  агент-длительность ~12-20 рабочих дней с проверками.
- В конце Phase 7 — ПОЛНОСТЬЮ удалить любые упоминания
  React/TypeScript/Tauri-WebView из репозитория. Никаких следов.
  v0.2.0 — чистый Slint + Rust.
- БЕЗ марафона. Никаких пачек по 29 релизов в час.
- Каждая фаза = один атомарный коммит со full 6-layer проверкой:
    1. cargo clippy --all-targets -- -D warnings
    2. cargo test --workspace
    3. Slint MCP tests для затронутых компонентов
    4. Agent review-agent (Agent tool, general-purpose, prompt =
       docs/REVIEW_AGENT_PROMPT.md adapted to Slint context) ПЕРЕД
       commit
    5. Live spawn через `cargo run` + визуальная проверка (через
       Slint MCP когда поднят, иначе ручной screenshot)
    6. git commit + push (git-gate.ps1 заблокирует если 1-3 красные;
       требует restart Claude Code чтобы стать активным)

БРЕНЧИНГ:
- Phase 0 пилот → ветка experiment/slint-replay
- Phase 1-7 → ветка slint/main (после go/no-go gate из пилота)
- master остаётся React+Tauri до Phase 7 cut, тогда сливаем slint/main
  в master force-or-merge как решит пользователь

ОБЯЗАТЕЛЬНО спроси пользователя ПЕРЕД:
- Phase 0 → Phase 1 переходом (go/no-go gate, представить
  docs/PILOT-REPORT-SLINT.md с метриками)
- Принятием Slint license tier — royalty-free (free + attribution
  badge in About) vs commercial (paid, no attribution)
- Любым решением, удаляющим или меняющим существующую UX-фичу
  (например: дропнуть translate tile feature вместо порта)

НЕ спрашивай — просто делай:
- Технические детали реализации (rust patterns, slint syntax, file
  layout)
- Refactors backend модулей которые остаются неизменными
  (audio/stt/ai/runtime/kb/journal/screenshot/hotkeys/config)
- Mкадетanically выглядящие commit сообщения

ВАЖНЫЕ ОГРАНИЧЕНИЯ:
- Сразу запиши решение по license tier в docs/ADR-002-license.md
  при первом upgrade в Phase 0 (default: royalty-free для pet-проекта,
  спроси если сомневаешься).
- Установленная binary версия у пользователя сейчас v0.1.1 React/Tauri
  — её НЕ ломаем до Phase 7. master продолжает работать.
- Multi-monitor особенность пользователя: primary 1920x1080 + secondary
  PORTRAIT 1200x1920 at x=-1200 (см. memory user-setup-monitors).
  Каждое окно по умолчанию на primary, апгрейд на non-primary только
  если landscape + >= primary width.
- WebView2 paint-баги решаются native Slint рендерингом — это
  главная мотивация миграции (см. memory no-marathon-releases).

СТАРТУЙ С PHASE 0 ПИЛОТА БЕЗ ПРЕДВАРИТЕЛЬНЫХ ВОПРОСОВ:
1. git checkout -b experiment/slint-replay
2. Создать docs/ADR-002-license.md (royalty-free по умолчанию)
3. Day 1 задачи из docs/MIGRATION-PLAN-SLINT.md § Phase 0:
   - Новый workspace member slint-experiment/
   - Cargo.toml deps (slint=1.16, slint-build, i-slint-backend-testing
     с mcp feature)
   - build.rs + replay.slint hello-world
   - Smoke run
4. Day 2 задачи: backend wiring, render timeline
5. Day 3 задачи: 3 Slint MCP теста + docs/PILOT-REPORT-SLINT.md
6. ОСТАНОВИСЬ + спроси пользователя go/no-go на Phase 1

Каждая выполненная Day НЕ требует подтверждения пользователя — двигайся
непрерывно. Только финальный gate (go/no-go в конце Phase 0) — спроси.

Не задавай уточняющих вопросов до старта. План полный, читай и исполняй.
```

---

## Что произойдёт в следующей сессии

1. Future Claude прочитает все 13 файлов (memory auto-loads первые 2,
   остальные читает Read tool).
2. Создаст ветку `experiment/slint-replay`.
3. Выполнит Phase 0 за ~6-8 часов агент-времени (день 1, 2, 3 без
   человеко-задержек).
4. Скажет: "Phase 0 завершён, вот метрики, go/no-go?".
5. На "go" → переходит на `slint/main`, делает Phase 1-7 за ~10-15
   рабочих дней агент-времени.
6. Финал: v0.2.0 ship + master полностью на Slint.

## Если что-то пойдёт не так

- **Pilot fail** — Claude напишет PILOT-REPORT с причинами, остановит
  миграцию, обновит ADR-001 заблокировать decision на React.
- **Markdown adapter взрывает Phase 4** — Claude перейдёт на гибрид
  (React только для тайл-контента) согласно risk register плана.
- **License surprises** — Claude спросит при первом столкновении.

## Что НЕ делать в следующей сессии

- Не запускать "marathon" — методология запрещает (см. memory
  no-marathon-releases).
- Не править одновременно `master` и `slint/main` — это разные миры
  до Phase 7.
- Не пытаться compile-и-test-after-каждой-строчки — это не
  agent-эффективно. Группируй изменения в логические единицы по фазам.
