# Сводный отчет исследования и передачи сессии (Handover Summary)
**Дата:** 2026-08-16
**Ветка задачи:** `codex/research-reconciliation`
**Базовый зафиксированный коммит (frozen baseline):** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`
**Текущий коммит ветки (HEAD):** `78aeac4b0a8a690e6fff07f36b6d76d4b0a8dfa0`
**Статус дерева:** Чистое (clean, за исключением локальной директории `docs/audit-grok/`, содержащей конфиденциальные нередактированные артефакты)
**Общий тестовый набор:** 284 исследовательских теста (operations) + 3 Hermes-интеграционных теста (0 skips, 100% pass)

---

## 1. Контекст задачи и ограничения

Задача заключается в проведении исчерпывающего, объективного аудита кодовой базы Suflyor, устранении ложных заявлений («100% покрытие», «watchdog автономен») предыдущих сессий, строгой пропорциональной верификации 119 оригинальных заявлений/гипотез аудита Grok, фиксации границ контрактов и подготовке точной карты кода.

### Строгие инварианты сессии:
1. **Никаких деструктивных действий и изменений прод-кода:** код в `overlay-backend`, `slint-experiment`, `suflyor-*`, `scripts` на этом этапе не модифицируется под видом «фиксов».
2. **Никаких тяжелых сборок Cargo/Swift/SDK на DSH-контроллере:** компиляция и нативные тесты выполняются только на целевых воркерах homelab (`windows-worker`, `mac-worker`) по строгим процедурам exact-SHA. На DSH запускаются только тесты парсеров, схемы, изолированные Python-модели и Git-архивные проверки.
3. **Изоляция моделей:** запрещены несанкционированные вызовы Astra, новые Grok-инференсы и самоделегирование.
4. **Не трогать `docs/audit-grok/`:** исходные файлы содержат сырые дампы с приватными путями/данными; для работы используются обезличенные данные в `docs/agent-map/reconciliation/grok-redacted/` и реестр `candidates.json`.
5. **Не закрывать цель (Goal) преждевременно:** цель остаётся активной (`armed` / `disarmed`), пока не будет получена реальная независимая приёмка и полное покрытие.

---

## 2. Что сделано: ключевые результаты по этапам (Rounds 1–36)

### А. Мультиязычный синтаксический индекс (Polyglot Syntax Coverage)
- **Охват файлов:** 298 выбранных исходных файлов распарсены без ошибок (0 parse errors):
  - 220 Rust
  - 4 Python
  - 25 Slint (через прекомпилированную библиотеку грамматики `libtree_sitter_slint.so`, ABI 15)
  - 5 Swift
  - 8 Objective-C
  - 1 C
  - 10 Bash
  - 24 PowerShell (через официальный переносимый PowerShell 7.4.13 AST-парсер `System.Management.Automation.Language.Parser` — решены все 7 ошибок парсера Tree-sitter без модификации скриптов)
  - 1 NSIS (через WASM-парсер `tree-sitter-nsis 0.4.1` под Node 22)
- **Навигационный реестр (`declarations.jsonl`, `files.json`):** 16 255 навигационных узлов. Зафиксировано: это навигационный индекс деклараций, а не «число функций» и не доказательство семантического аудита строк.

### Б. Архитектурные и функциональные контракты (26 Feature Contracts, 709 Source Ranges)
В директории `docs/agent-map/features/` зафиксированы строгие контракты со ссылками на строки и описанием границ применимости:
1. `stt-diarization-and-session-lifecycle.md`
2. `ai-and-vision-routing.md`
3. `read-aloud-and-ocr.md`
4. `personal-memory-lifecycle.md`
5. `settings-and-token-lifecycle.md`
6. `global-hotkeys-and-capture.md`
7. `knowledge-base-search.md`
8. `archive-playback-and-retranscription.md`
9. `update-and-release-lifecycle.md`
10. `mlx-sidecar-and-macos-lifecycle.md`
11. `hermes-protocol-and-sidecar.md`
12. `session-coaching-and-debrief.md`
13. `window-geometry-and-transparency.md`
14. `speech-activity-and-recording.md`
15. `startup-wizard-health-and-diagnostics.md`
16. `config-ui-translation-and-assets.md`
17. `native-macos-ffi-and-ownership.md`
18. `windows-capture-tray-and-sdk-ownership.md`
19. `credentials-and-managed-process-ownership.md`
20. `ui-setting-consumers-and-save-boundaries.md`
21. `wsola-streaming-and-playback.md`
22. `tera-graph-text-and-cancellation.md`
23. `overlay-tile-state-and-stream-terminals.md`
24. `archive-ui-confirmations-and-latches.md`
25. `audio-route-settings-and-watchdog.md`
26. И сопутствующие контракты подсистем.

### В. Реестр 119 оригинальных претензий Grok (Reconciliation Triage)
Статусы строго классифицированы и зафиксированы в `candidates.json`:
- **39 подтверждённых механизмов (confirmed):** подтверждено исходным кодом наличие указанного поведения (но без необоснованного объявления каждого пункта катастрофическим багом в проде).
- **75 гипотез (hypothesis):** требуют воспроизведения в нативной среде/условиях гонки, либо являются теоретическими рисками/артефактами крайних случаев.
- **5 отклоненных претензий (rejected):** опровергнуты кодом (например, безопасность буфера обмена, дублирование очистки каталогов и др.).

### Г. Серия изолированных фикстур и верификационных квитанций (Rounds 20–36)
В ходе последних раундов были созданы воспроизводимые, чистые математические, файловые и SQLite-модели для проверки ключевых гипотез:
1. **SQLite Contention (`hypothesis-sqlite-contention.md`, C02/C18):**
   - На реальном движке SQLite 3.45.1 и официальных миграциях проекта проверено: `BEGIN DEFERRED` при апгрейде чтения до записи после коммита другой транзакции даёт `SQLITE_BUSY_SNAPSHOT (517)`.
   - Запись первым потоком даёт обычный `SQLITE_BUSY (5)`.
   - Наличие активного читателя удерживает снимок WAL и блокирует `TRUNCATE`, но после отката читателя `TRUNCATE` обнуляет WAL до 0 байт.
   - По умолчанию включен `wal_autocheckpoint = 1000`.
2. **Journal Write & Identity (`hypothesis-journal-write-identity.md`, C05/C06):**
   - Воркер журнала ловит ошибку записи строки, сохраняет `first_error` и продолжает работу (`flush` возвращает ошибку в конце).
   - Повреждённая строка (незакрытый JSON) при чтении объединяется со следующей физической строкой, пропуская её, но последующие корректные JSON-строки успешно считываются.
   - Имя сессии формируется из секундной метки и младших 24 бит миллисекунд; открытие идёт в режиме `.append(true)`, а не `create_new`, что теоретически допускает слияние записей при коллизии миллисекунд.
3. **Journal Projection & Backfill (`hypothesis-journal-projection.md`, C08/C09):**
   - Пропущенный `session_stop` классифицирует сессию как `crashed`. При появлении `session_stop` последующий прогон исцеляет статус до `completed`.
   - `reindex_default` явно вызывает `store.backfill_session_models()` после `index_all()`, опровергая утверждение о том, что backfill никогда не вызывается.
4. **Side Tables & Diarization (`hypothesis-catalog-side-tables.md`, C10/C12, C13):**
   - `replace_session` удаляет сессию и каскадирует utterances/AI turns, но не затрагивает таблицы `diarization` и `memory_items`.
   - `delete_session` явно удаляет строку `diarization`.
   - `get_diarization` возвращает `Err` при повреждённом JSON сегментов (вопреки doc-комментарию о `None`).
   - `rename_speaker` выполняет чтение и полную замену JSON-блока без транзакции, из-за чего параллельная запись может затереть имя другого спикера.
5. **Memory Migrations & Grounding (`hypothesis-memory-migrations.md`, C14/C15; `hypothesis-memory-matching.md`, C04/C07; `hypothesis-memory-budget.md`, C01/C02; `hypothesis-memory-grounding.md`, C05/C08):**
   - В схеме `memory_candidates` колонка `status` не имеет ограничения `CHECK` и принимает произвольные строки. Повторное одобрение одного и того же снимка создаёт дубликат элемента.
   - Таблица `memory_items` не имеет внешнего ключа (FK) на `sessions` и сохраняется при удалении сессии.
   - Алгоритм `words_match` считает корнем общий префикс длиной `>= min(len, 4)` (что связывает антонимы вроде `проверили`/`провалили`).
   - `term_in_tokens` для терминов `>= 5` символов отсекает последнюю букву; латинские слова внутри кириллицы не имеют минимального порога длины.
   - Фильтр инструкций проверяет точные подстроки в нижнем регистре на пути Ask, но не вызывается на пути Summary.
   - Проверка бюджета отсекает только элементы после первого принятого.
   - `grounded_in_order` разрешает пропуск промежуточных слов при сохранении порядка; жесткая обрезка до 240 символов режет строку без учета границ слов.
6. **POSIX Credentials (`hypothesis-credential-files.md`, C05/C07):**
   - Проверка прав директории выполняется через `metadata` + `chmod 0o700` без `O_NOFOLLOW`.
   - Запись выполняется через временный файл `credentials.json.tmp` и `rename`, что при наличии симлинка на `.tmp` приводит к перезаписи целевого файла.
   - Ошибка парсинга JSON в файле учётных данных сбрасывает карту в пустую, что приводит к потере остальных слотов при сохранении.
7. **CI/CD & Installer (`hypothesis-gate-classification.md`, C04/C05; `hypothesis-installer-boundaries.md`, C06/C07; `hypothesis-ci-cleanup-gate.md`, C08/C10; `hypothesis-ci-version-security.md`, C12/C13):**
   - При отсутствии `origin/master` базой сравнения становится `HEAD~1`, из-за чего push из двух коммитов (`feat` + `docs`) может быть ошибочно классифицирован как docs-only.
   - `git diff --name-only` экранирует нестандартные символы кавычками, ломая суффиксную проверку.
   - Инсталлятор NSIS работает в контексте пользователя (`RequestExecutionLevel user`), устанавливает файлы в `%LOCALAPPDATA%`, не имеет подписи Authenticode и ACL-ограничений.
   - При удалении выполняется рекурсивный `RMDir /r` по путям в `%APPDATA%` и профиле без защиты от reparse points/junctions.
   - Основной workflow `ci.yml` содержит job `gate`, который не блокируется падением macOS или security-сканов.
   - Версии в `Cargo.toml` и `slint-installer.nsi` синхронизированы вручную (`0.38.1-rc.4`), но скрипт сборки не инжектирует версию в компилятор NSIS автоматически.
8. **Audio Clocks & Route Recovery (`hypothesis-audio-clock-route.md`, C02/C04):**
   - Метки времени аудио-чанков формируются на основе `Instant::elapsed()`, а не аппаратных счетчиков семплов.
   - Нецелочисленный коэффициент ресемплинга отбрасывает остаток в конце блока.
   - Политика восстановления WASAPI реагирует только на смену устройства по умолчанию с ролью `Console`; роли `Communications` и событие `OnDeviceAdded` игнорируются; pinned-устройства не переключаются на default.

---

## 3. Точный перечень ключевых файлов и артефактов

### Документация и отчеты:
- `docs/agent-map/README.md` — общая сводка структуры агентной карты.
- `docs/agent-map/reconciliation/NAVIGATION-GAPS.md` — детальный аудит навигационных пробелов (162 файла с точными ссылками, 136 без точных ссылок, 39 935 строк в объединении диапазонов).
- `docs/agent-map/reconciliation/progress.json` — машиночитаемый журнал прогресса (346 строк, точные счетчики, хэши проверок).
- `docs/agent-map/reconciliation/candidates.json` — 119Groks кандидатов с исходными цитатами и классификацией.
- `docs/agent-map/reconciliation/VERIFICATION.md` — сводный журнал верификации раундов 1–36.
- `docs/agent-map/reconciliation/NEXT-STEPS.md` — приоритизированная очередь дальнейших шагов.
- `docs/agent-map/reconciliation/hypothesis-*.md` — 18 детализированных отчетов по верифицированным гипотезам.

### Исполняемые тесты и фикстуры (`docs/agent-map/operations/`):
- `test_sqlite_contention_hypotheses.py`
- `test_journal_write_identity_hypotheses.py`
- `test_journal_projection_hypotheses.py`
- `test_catalog_side_table_hypotheses.py`
- `test_memory_migration_hypotheses.py`
- `test_journal_retention_utf8_hypotheses.py`
- `test_catalog_backup_recovery_hypotheses.py`
- `test_gate_classification_hypotheses.py`
- `test_installer_boundary_hypotheses.py`
- `test_ci_cleanup_gate_hypotheses.py`
- `test_ci_version_security_hypotheses.py`
- `test_memory_matching_hypotheses.py`
- `test_memory_budget_hypotheses.py`
- `test_memory_grounding_hypotheses.py`
- `test_diarization_rename_hypotheses.py`
- `test_credential_file_hypotheses.py`
- `test_audio_clock_route_hypotheses.py`
- А также вспомогательные скрипты `coverage_gaps.py`, `checkpoint.py`, `verify_portable_receipts.py`.

### Портативные квитанции (`docs/agent-map/reconciliation/portable-recovery-*.json`):
- Каждая квитанция привязана к точному SHA коммита, содержит хэш `navigation-gaps.json`, статус тестов и строгие флаги ограничений (`native_application_acceptance: false`, `independent_acceptance: false`).

---

## 4. Что остаётся сделать следующему агенту

1. **Не закрывать Goal:** цель `goal-fa8b2b4a-af0e-4ee1-b006-98a9b46c873c` должна оставаться в статусе незавершённой до выполнения всех условий.
2. **Продолжить покрытие оставшихся гипотез:**
   - В семействе `audio`: C05 (macOS detach retry), C06 (PTT f32 accumulator), C08 (loopback gaps padding).
   - В семействе `installers`: сетевые протоколы, редиректы, staging загрузок.
   - В семействе `config`: отсутствие зануления секретов в памяти (String zeroization).
3. **Сокращение навигационных пробелов (Navigation Gaps):**
   - Согласно `NAVIGATION-GAPS.md`, 136 файлов пока не имеют точных зарегистрированных интервалов в feature contracts (в основном тестовые модули, вспомогательные утилиты UI и глубокие ветки backend). Добавлять ссылки следует только на реально проанализированный код, без искусственного раздувания диапазонов.
4. **Независимая приёмка (Independent Acceptance):**
   - Текущие результаты получены координатором. Требуется провести независимую верификацию через отдельный рабочий маршрут Gemini/Opus без подмены моделей.
5. **Нативная приёмка (Native Acceptance):**
   - Выполняется на выделенных воркерах homelab (`windows-worker`, `mac-worker`) строго по процедурам `scripts/git-gate-native.ps1` против неизменяемого коммита в ветке задачи. Никаких сборок на DSH.

---
*Файл подготовлен для плавной передачи сессии следующей модели без потери контекста и нарушений установленных правил.*
