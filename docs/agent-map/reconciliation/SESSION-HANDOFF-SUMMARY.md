# Сводный отчет исследования и передачи сессии (Handover Summary)
**Дата:** 2026-08-16
**Ветка задачи:** `codex/research-reconciliation`
**Базовый зафиксированный коммит (frozen baseline):** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`
**Текущий коммит ветки (HEAD):** `1f09733b06e72d06db015f92104408973aa96091`
**Статус дерева:** Чистое (clean, за исключением локальной директории `docs/audit-grok/`, содержащей конфиденциальные нередактированные артефакты)
**Общий тестовый набор:** 423 исследовательских теста (operations) + 3 Hermes-интеграционных теста (0 skips, 100% pass)
**Статус покрытия гипотез из candidates.json:** 100% покрыты (0 uncovered hypotheses из 75)!

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

## 2. Ключевые результаты по верификации всех 75 гипотез Grok

Все 75 гипотез из `candidates.json` покрыты воспроизводимыми, изолированными тестами-моделями (`docs/agent-map/operations/test_*.py`) и запечатаны точными архивными квитанциями (`portable-recovery-*.json`).

### Основные охваченные области:
1. **SQLite Contention (`hypothesis-sqlite-contention.md`, C02/C18):**
   - На реальном SQLite 3.45.1 и официальных миграциях проверено поведение `BEGIN DEFERRED` при апгрейде чтения до записи (`SQLITE_BUSY_SNAPSHOT 517`) против обычной блокировки записи (`SQLITE_BUSY 5`).
   - Наличие активного читателя блокирует `TRUNCATE`, но после отката читателя `TRUNCATE` обнуляет WAL.
   - Подтверждён дефолтный `wal_autocheckpoint = 1000`.
2. **Journal Write & Identity (`hypothesis-journal-write-identity.md`, C05/C06):**
   - Воркер журнала сохраняет `first_error` и продолжает работу при ошибке строки (`flush` возвращает ошибку в конце).
   - Повреждённый JSON объединяется со следующей строкой, но последующие корректные JSON-строки считываются штатно.
   - Имя файла сессии формируется без флага эксклюзивного создания (`create_new`), используя 24-битный суффикс времени.
3. **Journal Projection & Backfill (`hypothesis-journal-projection.md`, C08/C09):**
   - Незавершённые сессии помечаются `crashed`, а появление `session_stop` в последующем прогоне восстанавливает статус `completed`.
   - `reindex_default` явно вызывает `store.backfill_session_models()` после `index_all()`.
4. **Side Tables & Diarization (`hypothesis-catalog-side-tables.md`, C10/C12; `hypothesis-diarization-rename.md`, C13):**
   - `replace_session` сохраняет таблицы `diarization` и `memory_items`, каскадируя только `utterances` и `ai_turns`.
   - `delete_session` явно удаляет запись в `diarization`.
   - `get_diarization` возвращает ошибку при невалидном JSON сегментов (вопреки doc-комментарию о `None`).
   - `rename_speaker` выполняет полную перезапись JSON-блока имён без транзакции, допуская потерю параллельных изменений.
5. **Memory Subsystem (`hypothesis-memory-*.md`, C01, C02, C04, C05, C07, C08, C14, C15):**
   - `memory_candidates.status` не имеет ограничения `CHECK`; повторный аппрув снимка создаёт дубликат записи.
   - `memory_items` не имеет внешнего ключа на `sessions` и сохраняется при удалении сессии.
   - `words_match` связывает слова по общему корню длиной `>= min(len, 4)` (включая пары антонимов вроде `проверили`/`провалили`).
   - `term_in_tokens` для слов `>= 5` букв отсекает последнюю букву; латинские слова внутри кириллицы не имеют минимальной длины.
   - Фильтр инструкций проверяет точные подстроки в нижнем регистре на пути Ask, но не вызывается на пути Summary.
   - Бюджет промпта останавливает добавление элементов только после первого принятого.
   - `grounded_in_order` разрешает пропуск промежуточных слов при сохранении порядка; жесткая обрезка до 240 символов режет строку без учета границ слов.
6. **POSIX Credentials (`hypothesis-credential-files.md`, C05/C07; `hypothesis-secret-zeroization-macos-recovery.md`, C06/C05):**
   - Права директории проверяются через `metadata` + `chmod 0o700` без `O_NOFOLLOW`. Запись через `credentials.json.tmp` следует по симлинкам.
   - Ошибка парсинга JSON сбрасывает карту в пустую и приводит к потере остальных слотов.
   - В Windows вызов `CredFree` освобождает блоб без предварительного затирания нулями; `secret_redacted` очищает строки через `clear()`, сохраняя аллоцированную ёмкость в куче.
   - На macOS `reopen_system` бесконечно повторяет попытки перезапуска каждые 500 мс без `RetryResult::Stop`.
7. **CI/CD & Installer (`hypothesis-gate-classification.md`, C04/C05; `hypothesis-installer-boundaries.md`, C06/C07; `hypothesis-ci-cleanup-gate.md`, C08/C10; `hypothesis-ci-version-security.md`, C12/C13):**
   - При отсутствии `origin/master` базой сравнения становится `HEAD~1`, что может маскировать код при пуше нескольких коммитов.
   - `git diff --name-only` без `-z` экранирует спецсимволы кавычками.
   - Инсталлятор NSIS работает в контексте пользователя без подписи Authenticode и ACL-ограничений; при удалении рекурсивно удаляет три дерева в профиле без проверки junctions.
   - Job `gate` в GitHub Actions не блокируется падением macOS или security-сканов.
   - Версии в `Cargo.toml` и NSIS синхронизированы (`0.38.1-rc.4`), но скрипт сборки не инжектирует версию в NSIS автоматически.
8. **Installers & Downloads (`hypothesis-installer-security.md`, C01/C02; `hypothesis-installer-protocols.md`, C03/C04):**
   - `extract_tar_bz2` вызывает системный `tar` без флагов `--exclude` и канонической проверки путей; `ocr_install` распаковывает архив прямо в корень `%APPDATA%\suflyor`.
   - `verify_sha256` читает файл по пути, не удерживая открытый дескриптор перед распаковкой; `download_installer` пишет файл в `%TEMP%`, а `run_installer` запускает его без повторной проверки хэша.
   - `curl_download` проверяет только префикс `https://` и следует редиректам с `-L`, но не ограничивает протоколы флагами `--proto =https`. `reqwest::Client` в апдейтере не ограничивает редиректы.
   - `teratts_install` использует staging-директорию, маркер `manifest.json` и карантин, в то время как `ocr_install` распаковывает архив in-place.
9. **Audio Subsystem (`hypothesis-audio-clock-route.md`, C02/C04; `hypothesis-audio-ptt-recorder.md`, C06/C08):**
   - Метки времени чанков формируются на основе `Instant::elapsed()`, а не аппаратных счетчиков семплов. Нецелочисленный коэффициент ресемплинга отбрасывает остаток в конце блока.
   - Политика восстановления WASAPI реагирует только на смену устройства с ролью `Console`.
   - PTT-функции `record_source_until_stop` на Windows и macOS накапливают семплы в динамический вектор без жесткого лимита размера буфера.
   - `plan_pad` в рекордере ограничивает одиночные паузы 10 минутами (`MAX_PAD_SAMPLES`), впитывая избыток в `skew`, и гарантирует запись WAV строго вперед (нулевой паддинг при обратных метках).
10. **TTS & Sidecars (`hypothesis-tts-stdio-pipe.md`, C01/C02; `hypothesis-tts-process-protocol.md`, C03/C04):**
    - `Tts::speak` кодирует весь текст в base64 и отправляет одной строкой `SPEAK <b64>` без чанкинга; сайдкары читают строки через `BufRead::lines()` без ограничения длины.
    - Родительский процесс сразу порождает отдельный фоновый поток ОС для вычитывания stdout сайдкара, предотвращая взаимоблокировку пайпов.
    - Привязка к Windows `JobObject` выполняется только под `#[cfg(windows)]`; на POSIX `kill_on_drop` и сигналы гибели родителя отсутствуют; `Sidecar` не имеет `Drop` с вызовом `kill()`.
    - `PlaybackTracker` использует FIFO-очередь `pending`, извлекая элементы только при ошибках парсинга base64/UTF-8.
11. **Local AI Engine & Ports (`hypothesis-local-ai-*.md`, C02, C03, C04, C05, C06, C07, C08, C09, C10, C11):**
    - `verify_engine_runs` проверяет порт 8077 через общий `wait_ready`, не проверяя PID владельца слушателя.
    - `ensure_servers_for_route` добавляет дескриптор дочернего процесса сразу после `launch_hidden` без проверки привязки к сокету.
    - `stop_listener_on_port` парсит строки `netstat` по суффиксу `:{port}` (поддерживая IPv4 и `[::1]`) и считает чужими процессы с неразрешимым путём exe.
    - Хэш SHA-256 вычисляется только для 26B-модели; модели на 12B/4B и проектор зрения проверяются только по размеру и штампу сборки.
    - Адреса загрузки бинарников сервера проверяются по белому списку хостов GitHub, но сами бинарники не имеют криптографической проверки хэша или Authenticode.
    - Модели загружаются с изменяемых веток HuggingFace `/resolve/main/`, но валидируются установщиком по фиксированным константам SHA-256.
    - Для неопределённого профиля видеокарты (`Unknown`) аргументы `-ngl` и `-np 1` опускаются; нормализация VRAM сглаживает только значения в окрестности 8/12/16 ГБ; `context_tokens` игнорирует флаг `_prep`.
    - Привязка серверов к `JobObject` выполняется только на Windows с флагом `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; готовность Whisper проверяется по коду возврата `curl` без валидации структуры ответа и PID.
12. **Session Bridge, Stealth & DWM (`hypothesis-bridge-*.md`, C02, C03, C04, C05, C06, C07, C08; `hypothesis-window-*.md`, C02, C04, C05, C06, C07, C08, C09, C10; `hypothesis-settings-*.md`, C02, C03, C04, C07, C09; `hypothesis-privacy-*.md`, C04, C05; `hypothesis-stt-*.md`, C03, C04):**
    - `maybe_spawn_auto_tile` изменяет состояние лимитов до вызова ИИ и проверяет поколение только после завершения запроса.
    - Прерывание пересыльщика транскриптов при остановке сессии может приводить к передаче уже инкрементированного номера поколения в задачу авто-тайла.
    - `try_acquire_auto_tile` разрешает вытеснение активного разрешения задачей более нового поколения сессии; ряд задач (`forward_audio_chunks`, дебрифинг) отбрасывает `JoinHandle`.
    - Мьютекс `rt` захватывается дважды на каждый аудио-чанк и удерживается при очистке буферов сессии и сортировке кэша ответов.
    - Дебрифинг сессии не передаёт поколение `session_gen`, в отличие от именования сессий `session_namer.rs`.
    - Окна паркуются по координатам `(-32000, -32000)` до вызова `show()`; защита от захвата экрана применяется до перемещения в видимую область; реестр окон не включает `TrayMenuWindow`.
    - `set_skip_taskbar` при изменении стилей принудительно отображает окно через `SW_SHOWNOACTIVATE`; центрирование оверлей-бара при ширине 0 сдвигает его на середину монитора.
    - `pick_monitor` требует наличия основного монитора; упаковка `TILE_MONITOR_PIN` при `left == i32::MIN` даёт коллизию со служебным маркером `AUTO`; вызовы снятия перехвата `RemoveWindowSubclass` отсутствуют.
    - `apply_transparency` расширяет рамку DWM на клиентскую область и включает blur-behind без `SetLayeredWindowAttributes`, очищая кнопки заголовка из стиля окна.
    - `fetch_models` опрашивает эндпоинты без проверки поколения; `open_settings` автоматически сохраняет конфигурацию Codex на диск при пустых полях.
    - `populate_token_status` безусловно сбрасывает флаги установки компонентов (кроме GigaAM) и оставляет неинициализированными свойства профиля модели и доступности зрения.
    - Старт и остановка сессии содержат синхронные блокирующие вызовы (валидация модели, захват звука, ожидание закрытия журнала до 3 с); тесты соединений выводят сырые цепочки ошибок, ограниченные 90 символами.
    - Преобразование снимка экрана выполняется в памяти без сохранения на диск; журнал хранит флаг `attached_screenshot: true` без растра; буфер обмена восстанавливает одиночный снимок без чтения истории.
    - Модель GigaAM кэшируется в общепроцессном мьютексе и загружается синхронно при валидации; сбой загрузки отбрасывает аудио-чанки без перехода в облако; диаризация блокирует поток без таймаута с лимитом аудио в 3 часа.

---

## 3. Точный перечень ключевых файлов и артефактов

### Документация и отчеты:
- `docs/agent-map/README.md` — общая сводка структуры агентной карты.
- `docs/agent-map/reconciliation/NAVIGATION-GAPS.md` — детальный аудит навигационных пробелов (162 файла с точными ссылками, 136 без точных ссылок, 39 935 строк в объединении диапазонов).
- `docs/agent-map/reconciliation/progress.json` — машиночитаемый журнал прогресса (346+ строк, точные счетчики, хэши проверок).
- `docs/agent-map/reconciliation/candidates.json` — 119Groks кандидатов с исходными цитатами и классификацией.
- `docs/agent-map/reconciliation/VERIFICATION.md` — сводный журнал верификации всех раундов.
- `docs/agent-map/reconciliation/NEXT-STEPS.md` — приоритизированная очередь дальнейших шагов.
- `docs/agent-map/reconciliation/hypothesis-*.md` — 25+ детализированных отчетов по верифицированным гипотезам.

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
- `test_installer_security_hypotheses.py`
- `test_installer_protocol_staging_hypotheses.py`
- `test_audio_ptt_recorder_hypotheses.py`
- `test_secret_zeroization_macos_recovery_hypotheses.py`
- `test_tts_stdio_pipe_hypotheses.py`
- `test_tts_process_protocol_hypotheses.py`
- `test_local_ai_engine_port_hypotheses.py`
- `test_local_ai_netstat_model_sha_hypotheses.py`
- `test_local_ai_download_artifacts_hypotheses.py`
- `test_local_ai_job_whisper_readiness_hypotheses.py`
- `test_local_ai_hardware_context_hypotheses.py`
- `test_bridge_generation_fence_hypotheses.py`
- `test_bridge_single_flight_cleanup_hypotheses.py`
- `test_bridge_contention_identity_hypotheses.py`
- `test_window_stealth_lifecycle_hypotheses.py`
- `test_window_geometry_taskbar_hypotheses.py`
- `test_window_monitor_subclass_hypotheses.py`
- `test_settings_models_codex_hypotheses.py`
- `test_settings_reset_seeding_hypotheses.py`
- `test_ui_blocking_and_error_leak_hypotheses.py`
- `test_privacy_capture_clipboard_hypotheses.py`
- `test_window_dwm_transparency_hypotheses.py`
- `test_stt_diarization_runtime_hypotheses.py`
- `test_overlay_tile_state_contract.py`
- А также вспомогательные скрипты `coverage_gaps.py`, `checkpoint.py`, `verify_portable_receipts.py`.

### Портативные квитанции (`docs/agent-map/reconciliation/portable-recovery-*.json`):
- Каждая квитанция привязана к точному SHA коммита, содержит хэш `navigation-gaps.json`, статус тестов и строгие флаги ограничений (`native_application_acceptance: false`, `independent_acceptance: false`).

---

## 4. Что остаётся сделать следующему агенту

1. **Не закрывать Goal:** цель `goal-fa8b2b4a-af0e-4ee1-b006-98a9b46c873c` должна оставаться в статусе незавершённой до выполнения всех условий.
2. **Все 75 гипотез теперь покрыты изолированными тестами.** Следующий исследовательский шаг — верификация оставшихся 33 подтверждённых механизмов (confirmed), у которых пока нет собственных отдельных тестов-моделей в `operations/`.
3. **Сокращение навигационных пробелов (Navigation Gaps):**
   - Согласно `NAVIGATION-GAPS.md`, 136 файлов пока не имеют точных зарегистрированных интервалов в feature contracts (в основном тестовые модули, вспомогательные утилиты UI и глубокие ветки backend). Добавлять ссылки следует только на реально проанализированный код, без искусственного раздувания диапазонов.
4. **Независимая приёмка (Independent Acceptance):**
   - Текущие результаты получены координатором. Требуется провести независимую верификацию через отдельный рабочий маршрут Gemini/Opus без подмены моделей.
5. **Нативная приёмка (Native Acceptance):**
   - Выполняется на выделенных воркерах homelab (`windows-worker`, `mac-worker`) строго по процедурам `scripts/git-gate-native.ps1` против неизменяемого коммита в ветке задачи. Никаких сборок на DSH.

---
*Файл подготовлен для плавной передачи сессии следующей модели без потери контекста и нарушений установленных правил.*
