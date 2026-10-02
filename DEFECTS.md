# Журнал дефектов и рисков Suflyor (DEFECTS.md)
**Дата актуализации:** 2026-10-02
**Правило шлюза:** Открытые дефекты P0 и P1 блокируют релиз, если на них нет явного письменного исключения (waiver) от владельца.

---

## 1. Критические дефекты (P0 — Блокируют работу / Безопасность / Сбои процессов)

| ID | Компонент / Файл | Описание проблемы | Статус | Связанный тест / PR |
|---|---|---|:---:|---|
| **DEF-01** | `.githooks/pre-commit:3` | Безусловный вызов `powershell.exe` роняет коммит-хук на macOS и Linux воркерах. | **Открыт** | Ветка `codex/fix-crossplatform-hooks` |
| **DEF-02** | `overlay-backend/src/ai.rs:188` | Самоблокировка (self-deadlock) в `complete_exclusive`: повторный захват мьютекса вызывающим потоком. | **Открыт** | Ветка `codex/fix-ai-self-deadlock` |
| **DEF-03** | `overlay-backend/src/credentials.rs:160` | На POSIX ошибка парсинга JSON сбрасывает файл в пустую карту, уничтожая остальные сохранённые ключи. | **Открыт** | Реестр `wave1_worker3_config-C05` |
| **DEF-04** | `overlay-backend/src/credentials.rs:140` | `credentials_path()` безусловно создаёт `suflyor/`, отсекая существующую папку данных `overlay-mvp/`. | **Открыт** | Реестр `wave1_worker3_config-C01` |
| **DEF-05** | `overlay-backend/src/local_ai.rs:912` | `is_reachable` использует `curl -s` без `--fail`, считая чужие процессы на порту 8080 живым llama-server. | **Открыт** | Реестр `wave2_worker4_local_ai-C01` |

---

## 2. Серьёзные дефекты (P1 — Утечки данных / Ошибки логики / Нарушения инвариантов)

| ID | Компонент / Файл | Описание проблемы | Статус | Связанный тест / PR |
|---|---|---|:---:|---|
| **DEF-06** | `slint-experiment/src/bin/overlay_host/diagnostics.rs:150` | Дефект двойного пробела после `Bearer `: токен не маскируется и утекает в лог экспорта. | **Открыт** | Реестр `wave4_worker1_privacy-C03` |
| **DEF-07** | `slint-experiment/src/bin/overlay_host/diagnostics.rs:117` | `redact_urls` ищет только `http://` и `https://`, пропуская протоколы `ws://`, `ftp://` и `file://`. | **Открыт** | Реестр `wave4_worker1_privacy-C02` |
| **DEF-08** | `overlay-backend/src/config.rs:1868` | `mask_host` маскирует узел, но сохраняет query-параметры (`?api_key=...`) и фрагменты URL без санитизации. | **Открыт** | Реестр `wave1_worker3_config-C02` |
| **DEF-09** | `overlay-backend/src/config.rs:957` | Отчёт `readiness` и предпросмотр настроек включают немаскированные пути и URL серверов. | **Открыт** | Реестр `wave1_worker3_config-C03` |
| **DEF-10** | `overlay-backend/src/config.rs:1580` | `save_to_path` пишет `.json.bak` без режима `mode(0o600)` на Unix, делая бэкап общедоступным. | **Открыт** | Реестр `wave1_worker3_config-C04` |
| **DEF-11** | `overlay-backend/src/config.rs:1390` | `preserve_corrupt_config` сохраняет файлы `json.broken-*` с секретами без маскирования и ротации. | **Открыт** | Реестр `wave1_worker3_config-C04` |
| **DEF-12** | `overlay-backend/src/config.rs:1696` | `merge_server_settings` и импорт затирают локальный путь `stt_gigaam_dir` чужими путями из файла. | **Открыт** | Реестр `wave1_worker3_config-C08` |
| **DEF-13** | `slint-experiment/src/bin/overlay_host/settings_controller.rs:700` | Полный импорт профиля замещает всю конфигурацию без фильтрации локальных путей текущего ПК. | **Открыт** | Реестр `wave3_worker4_settings-C08` |
| **DEF-14** | `slint-experiment/src/bin/overlay_host/settings_ai.rs:547` | Обработчики сохранения токенов игнорируют пустой ввод, делая невозможным стирание ключа из UI. | **Открыт** | Реестр `wave3_worker4_settings-C10` |
| **DEF-15** | `slint-experiment/src/bin/overlay_host/settings_ai.rs:745` | Изменение `ai_provider` в памяти не откатывается при последующей ошибке сохранения на диск. | **Открыт** | Реестр `wave3_worker4_settings-C05` |
| **DEF-16** | `slint-experiment/src/bin/overlay_host/settings_controller.rs:560` | Оптимистичное применение языка, монитора и stealth в памяти UI до подтверждения записи на диск. | **Открыт** | Реестр `wave3_worker4_settings-C06` |
| **DEF-17** | `slint-experiment/src/bin/overlay_host/settings_controller.rs:106` | Повторно используемое окно настроек не переинициализирует переключатели (коучинг, авто-тайлы, ретеншн). | **Открыт** | Реестр `wave3_worker4_settings-C01` |
| **DEF-18** | `slint-experiment/src/bin/overlay_host/window_lifecycle.rs:315` | `do_reveal` перемещает окно на экран даже при сбое `apply_stealth_one`, допуская утечку в запись. | **Открыт** | Реестр `wave3_worker2_window-C01` |
| **DEF-19** | `slint-experiment/src/bin/overlay_host/bar_tray.rs:454` | Резервный выход панели оверлея принудительно отображает окно на экране при включённом stealth. | **Открыт** | Реестр `wave3_worker2_window-C03` |
| **DEF-20** | `slint-experiment/src/bin/overlay_host/window_lifecycle.rs:78` | `STEALTH_EFFECTIVE` проверяет только рамку бара; ошибки скрытия тайлов не агрегируются. | **Открыт** | Реестр `wave3_worker2_window-C11` |

---

## 3. Умеренные дефекты (P2 — Устойчивость / Тесты / Инфраструктура)

| ID | Компонент / Файл | Описание проблемы | Статус | Связанный тест / PR |
|---|---|---|:---:|---|
| **DEF-21** | `scripts/git-gate-native.ps1:74` | Классификатор считает любые `.md` файлы документацией, пропуская компиляцию базы знаний `kb.rs`. | **Открыт** | Реестр `wave4_worker3_cicd-C01` |
| **DEF-22** | `scripts/git-gate-native.ps1:21` | `$crateOrder` не отслеживает сайдкар `suflyor-mlx` и вспомогательные модули. | **Открыт** | Реестр `wave4_worker3_cicd-C02` |
| **DEF-23** | `scripts/git-gate-native.ps1:151` | При UI-диффах в `slint-experiment` полностью пропускается `clippy` и запускаются только 13 тестов. | **Открыт** | Реестр `wave4_worker3_cicd-C03` |
| **DEF-24** | `.github/workflows/ci.yml:70` | Использование плавающего тега `@stable` компилятора и непривязанных версий GitHub Actions. | **Открыт** | Реестр `wave4_worker3_cicd-C09` |
| **DEF-25** | `.github/workflows/ci.yml:83` | Тестовое задание Windows пропускает крейты `suflyor-teratts` и `suflyor-wsola`. | **Открыт** | Реестр `wave4_worker3_cicd-C11` |
| **DEF-26** | `overlay-backend/src/audio.rs:803` | Бессимптомный ресемплинг 3:1 сбрасывает остаточные сэмплы `len % 3` на каждом шаге дециматора. | **Открыт** | Реестр `wave2_worker1_audio-C01` |
| **DEF-27** | `overlay-backend/src/audio.rs:348` | Формат аудиоклиента WASAPI запрашивается с `autoconvert`, но не считывается обратно из драйвера. | **Открыт** | Реестр `wave2_worker1_audio-C07` |
| **DEF-28** | `overlay-backend/src/audio.rs:410` | Таймаут ожидания буфера `wait_for_event(250)` логирует heartbeat, но не перезапускает драйвер. | **Открыт** | Реестр `wave2_worker1_audio-C03` |
| **DEF-29** | `overlay-backend/src/audio.rs:154` | `start_capture` возвращает `Ok` немедленно, пока поток устройства спит в цикле попыток. | **Открыт** | Реестр `wave2_worker1_audio-C09` |
| **DEF-30** | `overlay-backend/src/stt.rs:513` | Вызов `tokio::spawn` выполняется до захвата разрешения семафора, накапливая задачи в памяти. | **Открыт** | Реестр `wave2_worker2_stt-C01` |
| **DEF-31** | `overlay-backend/src/stt.rs:845` | Повторы STT используют фиксированные экспоненциальные задержки без рандомизации (jitter). | **Открыт** | Реестр `wave2_worker2_stt-C02` |
| **DEF-32** | `overlay-backend/src/journal/writer.rs:266` | Писатель журнала вызывает `BufWriter::flush()` без синхронного `sync_all` / `fsync`. | **Открыт** | Реестр `wave1_worker1_persistence-C04` |
| **DEF-33** | `slint-experiment/src/markdown.rs:67` | `parse_streaming` перезапускает полный парсинг `pulldown-cmark` по всему тексту каждые 50 мс. | **Открыт** | Реестр `wave3_worker3_tile-C02` |
| **DEF-34** | `overlay-backend/src/memory/normalize.rs:375` | Список `NEGATIONS` содержит только русские слова; `validate_rewrite` проверяет только количество. | **Открыт** | Реестр `wave1_worker2_memory-C03` |
| **DEF-35** | `overlay-backend/src/memory/context_builder.rs:106` | Запросы короче 4 символов отбрасываются, и контекст безусловно подставляет 8 свежих фактов. | **Открыт** | Реестр `wave1_worker2_memory-C06` |

---

## 4. Незначительные замечания (P3 — Захламление / Косметика)

| ID | Компонент / Файл | Описание проблемы | Статус |
|---|---|---|:---:|
| **DEF-36** | `experiments/*/Cargo.lock` | 12 932 строки неуправляемых локфайлов в папках macOS-экспериментов. | **В процессе очистки** |
| **DEF-37** | `docs/` | 155 устаревших бинарных PNG-скриншотов и 74 устаревших HTML-чеклиста. | **В процессе очистки** |
| **DEF-38** | `.claude/` | 10 файлов устаревших хуков с абсолютными путями разработчика. | **В процессе очистки** |
