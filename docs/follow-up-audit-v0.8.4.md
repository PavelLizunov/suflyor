# Повторный аудит после v0.8.4

Дата среза: 2026-06-02.

Цель файла: backlog для исправлений после повторного прохода по Rust + Slint
приложению. Пункты сверху важнее нижних. Строки приблизительные: во время аудита
в рабочем дереве уже шли параллельные изменения.

## Контекст

Уже реализованы: мастер первого запуска, диагностика, F1-help, текстовый ask,
F8 Vision, локальный AI-инсталлятор, copy, emergency restart, cloud escalation,
swipe-up-to-lock, импорт серверных настроек и автообновление.

Активный WIP во время аудита затрагивал `overlay-backend/src/{ai,audio,config,kb,
local_ai,runtime,stt,update,vision}.rs` и `overlay-backend/Cargo.toml`. Перед
исправлением каждого пункта сначала проверить, не закрыт ли он свежим diff.

## P0 — исправить в первую очередь

### P0.1 Локальный AI может завершить чужой процесс на порту 8080

Файлы:

- `overlay-backend/src/local_ai.rs:298`
- `overlay-backend/src/local_ai.rs:391`
- `overlay-backend/src/local_ai.rs:438`

`stop_listener_on_port()` ищет любой LISTENING PID на `:8080` и выполняет
`taskkill /F /PID`. Проверки имени, полного пути и владельца процесса нет.

Риск стал выше после текущего WIP: завершение может происходить не только при
переустановке локального AI, но и при обычном запуске приложения, если найден
`mmproj` и на порту уже кто-то отвечает.

Что сделать:

- Получить путь exe по PID.
- Завершать автоматически только ранее запущенный приложением
  `llama-server.exe` из ожидаемой директории `suflyor-local-ai`.
- Для чужого процесса показать понятную ошибку конфликта порта и не трогать PID.
- Рассмотреть PID-файл с путем exe и метаданными запуска.

Критерии приемки:

- Чужой тестовый HTTP-сервер на `8080` остается жив.
- Собственный orphan `llama-server.exe` корректно заменяется.
- В UI видна причина, если порт занят.

### P0.2 Инсталлятор локального AI может показать успех при неготовой модели

Файлы:

- `overlay-backend/src/local_ai.rs:353`
- `overlay-backend/src/local_ai.rs:799`

`wait_ready()` best-effort: после таймаута функция ничего не возвращает, а
`install()` продолжает формировать успешный `LocalAiResult`. Для whisper-server
после запуска отдельной строгой readiness-проверки также нет.

Что сделать:

- Превратить `wait_ready()` в `Result`.
- Проверить JSON `/models`, наличие ожидаемой модели и завершение дочернего
  процесса раньше времени.
- После установки выполнить минимальный text completion, Vision-запрос и STT
  smoke request.
- Не сохранять local-конфиг как рабочий, пока обязательные проверки не прошли.

Критерии приемки:

- Заблокированный `8080`, упавший server и битая модель завершают установку
  ошибкой.
- Ошибка объясняет, какой именно компонент не поднялся.

### P0.3 Приложение закрывается даже при ошибке запуска установщика обновления

Файл:

- `slint-experiment/src/bin/overlay_host.rs:8167`

Результат `overlay_backend::update::run_installer(&path)` игнорируется, после
чего вызывается `slint::quit_event_loop()`.

Что сделать:

- Закрывать приложение только после успешного `spawn`.
- При ошибке оставить приложение открытым и показать статус в Settings.
- Записать ошибку в диагностический лог.

Критерий приемки:

- Если запуск `.exe` запрещен или файл удален перед запуском, приложение остается
  открытым и показывает ошибку.

## P1 — важные недочеты

### P1.1 Диагностика отстает от набора функций

Файлы:

- `overlay-backend/src/config.rs:496`
- `slint-experiment/ui/settings_panel.slint:656`
- `slint-experiment/src/bin/overlay_host.rs:7447`

Сейчас readiness покрывает AI, STT, микрофон, system audio и stealth. После
расширения приложения этого недостаточно.

Добавить строки:

- Vision: provider, endpoint, модель и пробный запрос.
- Global hotkeys: успешность регистрации каждой клавиши.
- Локальные серверы: PID, порт, модель, состояние `starting / warming / ready /
  error`.
- Свободное место перед установкой моделей.
- Запись конфига и доступность директории журналов.

Дополнительно:

- Кнопка `Скопировать диагностический отчет`.
- Отчет должен быть redacted: без bearer, API key, содержимого транскрипта,
  профилей и скриншотов.

### P1.2 Конфликт global hotkey виден только в логе

Файл:

- `slint-experiment/src/bin/overlay_host.rs:2405`

При ошибке регистрации пользователь видит лишь `eprintln!`. В `docs/GUIDE.md`
это уже описано как известное ограничение.

Что сделать:

- Сохранить результат регистрации каждой комбинации.
- Показывать конфликт в Diagnostics и, при первом запуске, в wizard summary.
- Не писать в UI обобщенное `hotkeys disabled`, если сломалась одна клавиша.

### P1.3 В Config остались legacy-поля, не совпадающие с фактическим поведением

Файл:

- `overlay-backend/src/config.rs:239`

Подозрительные поля:

- `hotkey_ask`
- `hotkey_screenshot`
- `hotkey_toggle_visibility`
- `hotkey_pause_audio`
- `manual_ask_mode`
- `custom_css`

Например, в defaults остались `F10/F11/F12`, а приложение регистрирует
фиксированные `F1/F3/F4/F6/F8/F9/Shift+F9`.

Что сделать:

- Либо подключить реальные настраиваемые hotkeys с валидацией и миграцией.
- Либо удалить мертвые поля через версионированную миграцию.
- Добавить `config_version`, чтобы следующие миграции были явными.

### P1.4 Поврежденный config может незаметно заменить рабочие настройки defaults

Файл:

- `overlay-backend/src/config.rs:1960`

Текущий WIP улучшает запись: BOM поддержан, save стал atomic. Но если JSON уже
поврежден вручную или после внешнего сбоя, `load()` только пишет warning и
продолжает с defaults. При следующем сохранении пользователь может потерять
ключи, профили и устройства.

Что сделать:

- Не перезаписывать поврежденный файл молча.
- Сохранять `config.json.broken-<timestamp>` и последний хороший `.bak`.
- Показать пользователю восстановимый warning.
- Добавить сценарий restore из backup.

### P1.5 Проверки целостности local-AI загрузок пока частичные

Файл:

- `overlay-backend/src/local_ai.rs:687`
- `overlay-backend/src/local_ai.rs:841`

Текущий WIP добавляет allow-list хостов для release zip и грубую HTML-проверку
малых файлов. Большие модели и повторно используемые файлы по-прежнему
принимаются в основном по размеру. Для upstream release zip также нет pinned
SHA-256.

Что сделать:

- Хранить pinned SHA-256 для моделей и projector.
- Проверять hash после скачивания и перед reuse.
- Для динамически выбираемых upstream zip получать digest или использовать
  собственный manifest разрешенных артефактов.
- Перед загрузкой проверять свободное место и показывать ожидаемый объем.

### P1.6 Updater: определить fail-closed политику для SHA-256

Файл:

- `overlay-backend/src/update.rs:143`
- `overlay-backend/src/update.rs:198`

Текущий WIP уже добавляет PE-header и SHA-256 через GitHub API `digest`. Это
хорошее усиление. Но если digest не удалось получить, код продолжает запуск по
size + `MZ` header.

Нужно осознанно выбрать политику:

- Для обычного режима безопаснее fail-closed: нет digest — нет автоустановки.
- Можно оставить кнопку открыть страницу релиза для ручной установки.
- Если оставить fail-open, явно показать пользователю предупреждение.

### P1.7 Импорт серверных настроек стоит сделать более управляемым

Файлы:

- `overlay-backend/src/config.rs:2037`
- `slint-experiment/ui/settings_panel.slint:851`

Server-only import уже есть и сохраняет локальные UI-поля. Но для переноса между
компьютерами полезнее явно показать, что именно будет применено.

Что сделать:

- Добавить отдельный `Export server settings`.
- Перед импортом показать preview: cloud endpoints, local endpoints, модели,
  ключи-present/not-present, локальные пути.
- Дать галочки: `cloud secrets`, `local endpoints`, `local model paths`.
- Сделать backup и rollback.
- Не копировать machine-local пути по умолчанию.

## P2 — проверить и подчистить

### P2.1 F8 Vision на мониторах с разным DPI

Файлы:

- `slint-experiment/src/bin/overlay_host.rs:4504`
- `slint-experiment/src/bin/overlay_host.rs:4525`
- `slint-experiment/ui/capture_overlay.slint:6`

Для всего virtual desktop используется один `window.scale_factor()`. На двух
мониторах с разным DPI это может неверно преобразовывать logical selection в
пиксели замороженного изображения.

Проверить вручную:

- primary 100%, secondary 125% и 150%;
- secondary слева от primary с отрицательным X;
- выделение области через границу мониторов;
- масштабирование Windows после перезапуска.

### P2.2 Документация отстала от приложения

Файлы:

- `README.md:5`
- `docs/GUIDE.md:39`

Обновить:

- README показывает `v0.8.3`, а Cargo и NSIS уже `0.8.4`.
- GUIDE утверждает, что автоустановки обновлений нет.
- GUIDE не описывает F1-help и swipe-up-to-lock.
- В privacy-разделе уточнить: text/STT могут быть local, но Vision настраивается
  отдельно и может отправлять скриншоты в cloud.

### P2.3 Formatter

Текущий `cargo fmt --check` показывает форматирование в
`overlay-backend/src/update.rs`. После завершения активного WIP прогнать fmt.

### P2.4 Размер orchestration-файла

Файл:

- `slint-experiment/src/bin/overlay_host.rs`

Файл вырос примерно до 8270 строк. Это уже повышает риск регрессий в shared
state, stealth и lifecycle окон.

Кандидаты на вынос:

- updater;
- diagnostics;
- first-run wizard;
- local-AI installer UI wiring;
- capture/Vision flow;
- hotkey registration;
- window presentation helpers.

### P2.5 Секреты в plaintext

Файл:

- `%APPDATA%\overlay-mvp\config.json`

Для pet-project это допустимо, но стоит принять решение явно. Вариант усиления:
Windows DPAPI для bearer/API keys, а экспорт — отдельным осознанным действием с
предупреждением.

### P2.6 Markdown-таблицы могут обрезаться

Файлы:

- `slint-experiment/src/markdown.rs:249`
- `slint-experiment/ui/tile.slint:434`

Таблицы теперь форматируются лучше, но широкий monospace-блок сознательно
клипается справа. Проверить реальные AI-ответы с 4-6 колонками. Возможные
варианты: горизонтальный scroll, компактный режим, раскрытие таблицы.

## Идеи на потом

Это не баги. Реализовывать только если идея подходит продукту.

### I1 Vision presets

Сейчас F8 использует фиксированный prompt (`overlay-backend/src/vision.rs:16`).
Перед отправкой можно предложить быстрый выбор:

- прочитать текст;
- объяснить схему;
- разобрать код;
- решить задачу;
- свой вопрос.

### I2 Восстановление после аварийного restart

После emergency restart предложить восстановить последнюю незавершенную сессию:
последний journal, transcript summary и контекст. Не восстанавливать
незавершенные сетевые запросы автоматически.

### I3 Явный cloud-egress индикатор

Перед первым cloud Vision вызовом показать одноразовое подтверждение. На тайле и
в capture overlay отображать короткий badge `cloud` или `local`.

### I4 Повторяемый smoke-check перед релизом

Сделать короткий сценарий проверки:

1. Первый запуск и wizard.
2. Diagnostics check-all.
3. Stealth preview для bar, tiles, F4, F1, Settings, Text Ask, Wizard и capture.
4. Hotkeys F1/F3/F4/F6/F8/F9/Shift+F9.
5. PTT + swipe-lock + stop.
6. Два монитора и mixed DPI.
7. Update spawn failure.
8. Local AI install, cancel, reinstall, occupied ports и orphan recovery.

## Проверки во время аудита

До последних параллельных изменений:

- `cargo test --manifest-path overlay-backend/Cargo.toml`:
  `172 passed`, `1 ignored`.
- `cargo test --manifest-path slint-experiment/Cargo.toml`:
  `33 passed`, `2 ignored`.
- `git diff --check`: чисто.

После завершения текущего WIP тесты нужно прогнать повторно: во время записи
этого файла менялись дополнительные backend-модули.
