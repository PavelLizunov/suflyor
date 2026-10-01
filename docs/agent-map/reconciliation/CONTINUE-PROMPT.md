# Промпт для продолжения Suflyor research

Продолжи незавершённое исследование Suflyor в `/var/lib/dsh/Project/suflyor`. Сначала проверь `pwd`, `git status --short --branch`, последние коммиты и remote. Наша task-ветка — `codex/research-reconciliation`, GitHub — `PavelLizunov/suflyor`. Не переключай/не перезаписывай чужую работу. Прочитай `AGENTS.md`, `docs/AGENTS.md`, `docs/goal-agent-map-reconciliation.md`, `docs/agent-map/README.md`, `docs/agent-map/reconciliation/progress.json`, `VERIFICATION.md`, `NEXT-STEPS.md` и этот handoff.

## Задача и ограничения

Пользователь хочет полную агентно-читаемую карту функций/фич/связей проекта и честную проверку старых исследований. Разрешены только явно выбранные Gemini/Opus workers; НЕ использовать Astra или новые Grok-вызовы. Gemini — только короткие полезные проверки; workers не делегируют. Производственный код на этом этапе не меняем. Не запускать Cargo/Swift/нативные сборки на DSH, не ставить SDK, не перезапускать DSH, не публиковать релиз/не merge. Любая нативная проверка — отдельная точная SHA и процедуры homelab/Slint QA.

## Реально сделано

- Исходный baseline зафиксирован: `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; source/raw хеши в `reconciliation/snapshot.json`. Точные CRLF/LF формы отдельно в `source-portability.json`.
- Предыдущий агент ложно объявил regex-индекс «100% построчным аудитом», локальный helper — автономным watchdog, а `changes_required` — завершением. Статусы исправлены; полная задача НЕ завершена.
- Все 119 оригинальных претензий Grok сопоставлены с точными текстами и исходниками: 39 подтверждённых механизмов, 75 гипотез, 5 отклонены. Это не 39 воспроизведённых багов. См. `candidates.json`, `coordinator-checks.json`, `SUMMARY.md`, `REMEDIATION.md`.
- Сырые 14 файлов `docs/audit-grok/` НЕ изменены и не закоммичены: есть private-network examples. Обезличенные копии и их provenance сохранены в `grok-redacted/`.
- 15 bounded feature contracts в `docs/agent-map/features/`, 410 source references: STT/диаризация/сессии, AI/vision/local, TTS/OCR, память, настройки, hotkeys/capture, KB/archive/re-STT/summary/coaching, updater/release, MLX/Hermes. Это principal chains, не вся семантика/все callers.
- GigaAM/Whisper распознают текст; managed Whisper — large-v3-turbo Q8. macOS GigaAM CoreML/CPU история сохранена. Новый Nemotron V3 — optional Windows speaker diarization, не замена STT; наблюдавшийся последний опубликованный RC предшествует этой реализации. Проверить актуальный release status, не верить version bump.
- Recovery helper не перезапускает сессию: сохраняет checkpoint, проверяет drift/original IDs, не принимает null/misaligned proposals и не повторяет unknown jobs. Последний exact Git archive `a8c752534ccba081aecc99040e4f03f54c65f47e`: 71+3 tests, zero skips, hashes/ranges/119 claims recovery, byte-identical regeneration всех syntax artifacts. Receipt `portable-recovery-round5.json`; parsers supplied отдельно из pinned ignored environment, не в Git. Старые receipts сохранены исторически.
- Round-5 syntax index расширен: 290 успешно parsed files (224 Rust/Python +25 Slint +5 Swift +8 Objective-C +1 C +10 Bash +17 PowerShell), ещё семь PowerShell файлов с явными parse_error/partial nodes, один NSIS unsupported, 22 protected/vendor excluded, 520 nonselected. 16,243 declaration/node records ВКЛЮЧАЯ прежние 7,692 unexpanded Rust macro invocations и Slint callback events/imports; НЕ количество функций и НЕ semantic acceptance. См. `syntax/README.md`, `files.json`, `declarations.jsonl`, `parser-polyglot-research.md`.
- Parser wheels: tree-sitter 0.25.2 + tree-sitter-rust 0.24.2, Python3.12/Linux x86_64; isolated ignored `.campaign-state/parser-site-stable`. Hash requirements/provenance в operations/reconciliation. 0.26.0 падал SIGSEGV; failure receipts сохранены, не повторять его. Парсинг по subprocess на файл, не SDK/production dependency.
- После startup/health slice local suite: 79 research tests (48 recovery/SQL/source +23 syntax/index fixtures +8 startup assertions); 16 bounded feature contracts /451 source ranges. Старый parser exact receipt имеет 71+3; последний startup exact receipt см. VERIFICATION. Все утверждения source-only, не independent/native acceptance. Parser suite без skips при всех pinned grammars. Binding остаётся 0.25.2; новые hash pins/provenance в `parser-polyglot-requirements-linux.txt` / `parser-polyglot-provenance.json`. Slint library — hash-gated precompiled release artifact, путь через SUFLYOR_RESEARCH_SLINT_GRAMMAR; binary не в Git. Без grammars skips НЕ PASS. Три mocked Hermes и exact-SHA portable receipts — отдельно в VERIFICATION. Нативное поведение/инференс/состав релиза/независимая приёмка НЕ проверены.

## Продолжение

1. Проверь актуальный HEAD/remote и последний manifest/progress/verification; коммиты handoff могут быть новее описанных baseline.
2. Собери/прочитай saved syntax index, запусти `python3 -B docs/agent-map/operations/syntax_index.py --validate` и checkpoint verify. Если нужна повторная генерация, сначала прочитай owning outputs и воспроизведи pinned wheel environment без изменения app/runtime.
3. Доделай NSIS и разреши семь PowerShell grammar-error файлов (это не доказательство ошибок скриптов); остальные языки имеют bounded CST index, не semantic/caller acceptance. Не изобретай regex, который снова объявит zero-symbol full coverage; не считай cfg/macros resolved.
4. Доделай startup/wizard/components/health/diagnostics, полную config/UI/translation/asset schema и оставшиеся native FFI/call edges. Сверяй callers: комментарии уже неоднократно были устаревшими.
5. Исследуй 75 гипотез пропорционально. Исторические O1–O8/модельные ответы — candidate evidence, не independent acceptance. Null Opus/Gemini receipts есть; повторение только при установленной причине и рабочем разрешённом маршруте.
6. Не делай production fixes массово из audit; каждый fix — scope/spec/точная проверка. Всё task-owned исследование коммитить/пушить coherent checkpoints в нашу ветку; не трогать master/force/чужой audit.

## Восстановление цели

В старой сессии goal `goal-beae4fa0-c797-45da-9b57-f3c47328f273` не завершён; работа остановлена только ради переноса контекста. В новом чате прочитай цель через get_goal, если доступна, и resume по этому прямому запросу; если goal не переносится — создай новый persisted goal с тем же результатом/ограничениями. Не отмечай complete до реального достижения всего согласованного объёма.
