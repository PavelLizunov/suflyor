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
- Recovery helper не перезапускает сессию: сохраняет checkpoint, проверяет drift/original IDs, не принимает null/misaligned proposals и не повторяет unknown jobs. Exact Git-archive receipts предыдущих 24/34/48+3 suites сохранены.
- Точный Rust/Python syntax index добавлен: 224 parsed files (220 Rust +4 Python), 13,961 declaration/node records ВКЛЮЧАЯ 7,692 unexpanded macro invocations; это НЕ количество функций. Ноль parse/native errors при accepted pinned run. 74 unsupported language files, 22 protected/vendor excluded, 520 nonselected. См. `syntax/README.md`, `files.json`, `declarations.jsonl`.
- Parser wheels: tree-sitter 0.25.2 + tree-sitter-rust 0.24.2, Python3.12/Linux x86_64; isolated ignored `.campaign-state/parser-site-stable`. Hash requirements/provenance в operations/reconciliation. 0.26.0 падал SIGSEGV; failure receipts сохранены, не повторять его. Парсинг по subprocess на файл, не SDK/production dependency.
- Последний локальный suite с parser-site: 62 research tests (22 recovery/provenance +7 SQL +19 source seams +14 syntax fixtures), плюс 3 existing mocked Hermes tests. Без parser wheels Rust fixtures skip — это НЕ PASS parser. Нативное поведение/инференс/состав релиза НЕ проверены.

## Продолжение

1. Проверь актуальный HEAD/remote и последний manifest/progress/verification; коммиты handoff могут быть новее описанных baseline.
2. Собери/прочитай saved syntax index, запусти `python3 -B docs/agent-map/operations/syntax_index.py --validate` и checkpoint verify. Если нужна повторная генерация, сначала прочитай owning outputs и воспроизведи pinned wheel environment без изменения app/runtime.
3. Доделай language-aware extraction для Slint/PowerShell/Bash/Swift/Objective-C/NSIS с parse errors и явными limits. Не изобретай regex, который снова объявит zero-symbol full coverage; не считай cfg/macros resolved.
4. Доделай startup/wizard/components/health/diagnostics, полную config/UI/translation/asset schema и оставшиеся native FFI/call edges. Сверяй callers: комментарии уже неоднократно были устаревшими.
5. Исследуй 75 гипотез пропорционально. Исторические O1–O8/модельные ответы — candidate evidence, не independent acceptance. Null Opus/Gemini receipts есть; повторение только при установленной причине и рабочем разрешённом маршруте.
6. Не делай production fixes массово из audit; каждый fix — scope/spec/точная проверка. Всё task-owned исследование коммитить/пушить coherent checkpoints в нашу ветку; не трогать master/force/чужой audit.

## Восстановление цели

В старой сессии goal `goal-beae4fa0-c797-45da-9b57-f3c47328f273` не завершён; работа остановлена только ради переноса контекста. В новом чате прочитай цель через get_goal, если доступна, и resume по этому прямому запросу; если goal не переносится — создай новый persisted goal с тем же результатом/ограничениями. Не отмечай complete до реального достижения всего согласованного объёма.
