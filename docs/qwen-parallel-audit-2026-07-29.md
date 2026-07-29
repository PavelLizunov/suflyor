# Параллельный Qwen-аудит Suflyor — 2026-07-29

## Итог

Через локально установленный Qwen Code CLI `0.21.0`
(`qwen3.8-max-preview`) параллельно выполнены 11 независимых статических
аудитов текущего checkout. Максимум одновременно работали 3 процесса.

- время успешного прогона: 34 минуты;
- сырые verdict: 10 `WARN`, 1 `FAIL` (Win32/stealth);
- сырые findings: 43;
- после ручной проверки по текущему коду: **32 confirmed, 10 partial,
  1 false**;
- 9 ответов были строгим JSON, 2 добавили текстовую преамбулу перед валидным
  JSON-объектом;
- исходные prompt/result/stderr и manifest оставлены в
  `%TEMP%\suflyor-qwen-audit-20260729-codex`.

Runner: `scripts/qwen-parallel-audit.ps1`. Он использует `--safe-mode`,
`--approval-mode plan`, `--no-chat-recording`; shell/edit/write/web/MCP/
subagent/computer-use инструменты исключены. Qwen мог только читать файлы
checkout через `read_file`, `grep_search`, `glob`, `list_directory`.

## Статус исправлений

Ремедиация ведётся отдельными минимальными изменениями. На 2026-07-29:

- [x] #1 — Hermes-секреты очищаются перед записью `config.json.bak` (PR #37);
- [x] #2 — pause отсекает аудиочанки до всех STT-провайдеров независимо от
  настройки записи (PR #39);
- [x] #3 — Error/незавершённый AI-stream больше не сохраняет частичный ответ
  как успешную Q&A-пару;
- [x] #6 — hard-delete удаляет связанные строки diarization (PR #36);
- [x] #16 — оставшиеся UI-строки переведены через `@tr` (PR #38);
- [~] #18 — tofu-глифы убраны из Slint, пользовательские Rust-строки с emoji
  ещё требуют очистки.
- [x] #20 — повторный approve кандидата памяти больше не создаёт дубликат
  `memory_items`;

Полностью закрыты 6 из 22 приоритетных корней, один закрыт частично. Остальные
15 приоритетных пунктов и low/conditional gaps остаются открытыми.

## Покрытие

| # | Домен | Raw | Проверено | Главный результат |
|---|---|---:|---:|---|
| 1 | Config / Settings / update | 3 | 1 C / 2 P | Hermes secrets попадают в `config.json.bak` |
| 2 | Audio / STT / recording | 3 | 2 C / 1 P | Pause не останавливает STT-провайдер |
| 3 | AI / bridge / local AI / Hermes | 4 | 3 C / 1 P | AI error журналируется как успешный `stop` |
| 4 | Ask / tile / conversation | 3 | 2 C / 1 P | Shift+F9 guard и PTT watchdog race |
| 5 | Win32 / windows / hotkeys | 6 | 5 C / 1 P | One-shot HWND/WDA и ложный stealth-state |
| 6 | Vision / OCR / TTS | 4 | 4 C | TTS pause/error lifecycle и clipboard data loss |
| 7 | Journal / archive / recovery | 4 | 3 C / 1 P | Hard-delete orphan и ложный crash при Quit |
| 8 | Diarization / re-transcribe | 3 | 2 C / 1 P | Неатомарная установка и потеря результата |
| 9 | KB / memory / context | 2 | 2 C | Повторный approve и SQLite на UI-thread |
| 10 | Slint UI / i18n / icons | 6 | 4 C / 1 P / 1 F | Голые строки и пробел в i18n guard |
| 11 | Build / installer / docs | 5 | 4 C / 1 P | Устаревшая release-инструкция и docs |

`C` = confirmed, `P` = partial, `F` = false.

## Подтверждённые приоритетные корни

### Security / privacy / data integrity

1. `overlay-backend/src/config.rs:1288` — `secret_redacted()` не очищает
   `hermes_bridge_token` и `hermes_api_key`; предыдущий config сериализуется
   в `config.json.bak`. Тест редактирования секретов эти два поля не покрывает.
2. При pause с выключенной записью аудиочанки продолжают уходить в STT, а
   готовые события отбрасываются только позднее. Для Groq это возможная
   отправка речи и расход квоты во время явной паузы.
3. `runtime.rs:1928` — Error-ветка оставляет `finish_reason = "stop"` и
   безусловно пишет `AiResponse`; journal может сохранить частичный текст как
   успешную Q&A-пару.
4. Win32 stealth применяется через one-shot получение HWND и почти везде
   игнорирует результат `set_stealth`. Bar, CaptureOverlay и tile могут
   остаться незащищёнными или припаркованными, пока UI продолжает показывать
   stealth включённым. Повторного применения/явного fail-state нет.
5. Ctrl+C capture сохраняет и восстанавливает только текст clipboard.
   Изображение или список файлов очищаются без возможности восстановления;
   единственная проверка нового владельца через 140 мс также даёт silent
   no-op на медленном приложении.
6. `Store::delete_session` удаляет session/FTS, но не строки `diarization`.
   Заявленный hard-delete оставляет сегменты и имена спикеров в
   `catalog.sqlite`.

### Reliability / lifecycle

7. Quit, restart и updater закрывают event loop без `stop_session`; активная
   сессия остаётся без `SessionStop` и показывается в архиве как crashed.
8. Установка diarization распаковывает архив прямо в рабочий каталог, а
   readiness проверяет только наличие `model.onnx`. Прерванная распаковка
   может оставить постоянное ложное состояние «установлено» без repair UI.
9. Закрытие transcript window уничтожает poll timer, но тяжёлый detached
   diarization продолжает работу. Результат больше никто не сохраняет, а после
   повторного открытия можно запустить второй sidecar.
10. Shift+F9 всегда выбирает cloud endpoint без проверки пустого bearer,
    поэтому штатный authenticated bridge не работает у local-only
    пользователя. Аналогичный guard уже есть у follow-up.
11. После PTT watchdog stop отпускание кнопки может немедленно начать новую
    запись. Старый результат не имеет generation-id и способен сбросить
    состояние/запустить лишний AI-вызов поверх нового.
12. TTS speaking deadline продолжает идти во время pause; после долгой паузы
    звук возобновляется, когда `is_speaking()` уже false, и STT не подавляет
    loopback/эхо.
13. `wire_speak()` выставляет `can_speak=true` без проверки sidecar/voice.
    `mark_speaking_for()` вызывается до подтверждения запуска, поэтому
    беззвучная ошибка может ложно подавлять STT на расчётную длительность.
14. `stream_chat` ждёт semaphore без cancellation закрытого receiver и может
    начать уже ненужный запрос после supersession/stop. Заявленная Qwen
    привязка к обычному закрытию F9/PTT tile была неточной, но общий
    cancellation gap подтверждён.
15. Settings включает WDA, но не восстанавливает `WS_EX_TOOLWINDOW`; после
    выключения stealth через bar и включения через Settings кнопка bar может
    остаться в taskbar.

### UI / correctness / maintainability

16. `text_ask.slint:124,151,156` содержит голые русские строки без `@tr`, а
    `overlay_bar.slint:979` — голую английскую `"AI streaming"`.
17. `i18n_guard.rs` проверяет только `@tr(...)` и не видит голые
    `text`/`placeholder-text` literals, поэтому пункт 16 проходит CI.
18. `transcript.slint:317,342` использует `Copied ✓`, нарушая проектное
    no-tofu правило. Emoji также остаются в memory/vision Rust-строках.
19. OCR/read-aloud пропускает дословный текст через markdown cleanup:
    `#include`, `int *p`, backticks и `~` озвучиваются с изменёнными
    символами.
20. `approve_candidate` повторно вставляет тот же факт, потому что не
    проверяет status и в схеме нет уникальной связи candidate→item.
21. `context_for_meeting` синхронно открывает SQLite и читает все active
    memory items на Slint event loop в ask/follow-up/regenerate/vision путях.
22. `RELEASE_CHECKLIST.md` всё ещё предлагает прямой push в `master`;
    `docs/architecture.md`, комментарии CI и README устарели относительно
    трёх crates и версии `0.34.0`; visual gate указывает неверный путь к
    capture scripts и старый title окна.

## Подтверждённые low/conditional gaps

- неудачный PTT mic acquire молча возвращается без сообщения «микрофон занят»;
- ошибка start capture может оставить пустую сессию со статусом crashed;
- raw Settings errors иногда показывают локальные пути; Qwen переоценил
  гарантированность username leak в import/export;
- local-AI cleanup выполняет `taskkill`/`wait()` под `AppState` mutex;
- `put_diarization()` error игнорируется, UI показывает несохранённый результат;
- ограниченный journal reindex retry может навсегда оставить SQLite status
  `crashed`, хотя JSONL позднее корректно допишется;
- deterministic same-millisecond journal collision возможен на уровне API, но
  достижимый production-сценарий не доказан;
- soft cost-cap parity для PTT неполна, однако сейчас cap event и в других
  путях только логируется, а не показывается пользователю;
- memory kind/vision emoji нарушают no-tofu правило; архивные emoji из Qwen
  оказались только устаревшим комментарием;
- README/release/architecture проблемы — документационные; исполняемый
  `scripts/ci.ps1` действительно проверяет все три crates.

## Опровергнуто

Qwen посчитал IPA-пример в Settings tofu-риском. Проверка установленного
`%WINDIR%\Fonts\segoeui.ttf` показала наличие `U+02C8`, `U+0283`, `U+02D0`;
finding исключён.

## Границы проверки

Это статический аудит. Qwen не запускал сборку, тесты, приложение, сеть и не
читал live config. Аудио, screen-share/WDA, clipboard races, DPI/clipping и
реальное поведение sidecar требуют отдельных runtime/manual ретестов.
Исправления в этом проходе не вносились.
