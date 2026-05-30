# Slint Surface Audit

Беглый, но более подробный осмотр Slint UI и Rust glue. Ничего в коде не исправлялось; это список шероховатостей, возможных багов и мест, которые стоит проверить руками.

## Вероятные баги / несоответствия

1. `ui/palette.slint:120` — UI обещает `Enter to spawn tile`, но `ui/palette.slint:71` обрабатывает только `Key.Escape`.
   - В `src/bin/overlay_host.rs:3643` есть `on_result_activated`, но в `.slint` он вызывается только кликом по результату.
   - Риск: пользователь открывает F4, печатает запрос, жмет Enter, а ничего не происходит.

2. `ui/palette.slint:45` и `ui/palette.slint:146` — есть `selected-index` и визуальное выделение результата, но нет обработки Up/Down.
   - `key-pressed` в `ui/palette.slint:71` не меняет `selected-index`.
   - Риск: выделение выглядит как keyboard navigation, но фактически почти декоративное.

3. `ui/settings_panel.slint:1237` — `stt-whisper-bearer-save` сохраняет bearer, но поле не очищается после Save.
   - Для сравнения: `ai-bearer-input` очищается в `ui/settings_panel.slint:942`, `ai-local-bearer-input` в `ui/settings_panel.slint:1077`, `groq-api-key-input` в `ui/settings_panel.slint:1283`.
   - Дополнительно `src/bin/overlay_host.rs:5342` при открытии Settings кладет сохраненный `stt_whisper_bearer` обратно в UI input.
   - Риск: секрет дольше живет в UI state и на экране в password-поле, поведение непоследовательно с остальными ключами.

4. `ui/tile.slint:364` — комментарий говорит, что code blocks должны иметь horizontal scroll и no-wrap, но фактический код в `ui/tile.slint:371` ставит `wrap: word-wrap`.
   - Риск: код в AI-ответах будет переноситься и терять форматирование, особенно команды, YAML, JSON, shell snippets.

5. `src/bin/overlay_host.rs:2023` — кнопка `+ tile` сейчас не делает реальный ask, если `SLINT_OVERLAY_DEMO` не включен.
   - В `src/bin/overlay_host.rs:2035` прямо выводится текст: demo disabled, Phase B2 will wire live mic transcript.
   - Риск: пользователь видит функциональную кнопку `+ tile`, но получает fallback/demo content. Если это сознательно оставлено, стоит переименовать кнопку или убрать production-like ожидание.

## UX-компромиссы

1. `ui/tile.slint:380` — таблицы рендерятся monospace блоком с `wrap: no-wrap`, а комментарий признает: wide tables clip on the right.
   - В `src/markdown.rs:232` таблица дополнительно режет ячейки до 28 символов.
   - Компромисс понятен, но для технических ответов таблицы могут быть важными; нужен горизонтальный scroll или адаптивный table view.

2. `src/markdown.rs:9` — markdown adapter урезан: bold/italic теряются, links/images/html silently dropped.
   - Это нормально для MVP, но AI-ответы часто используют ссылки, inline emphasis и списки с вложенностью.
   - Риск: ответ визуально беднее, чем ожидает пользователь от markdown.

3. `ui/replay.slint:33` — окно называется `Replay — Slint pilot (Phase 0)`.
   - Если Replay уже доступен пользователю, заголовок выглядит как внутренний/черновой.
   - `ui/replay.slint:1` также говорит `Replay viewer scaffold`, хотя backend уже, похоже, подключен через `replay_backend.rs`.

4. `ui/overlay_bar.slint:159` и дальше — bar имеет фиксированный `preferred-width: 1080px` и историю ручных расширений/сужений.
   - Риск на маленьких экранах/масштабировании/длинных переводах: снова возможен clipping.
   - Стоит проверить при 125-150% DPI и английском/русском UI.

5. `ui/settings_panel.slint:147` — Settings фиксированно 720x560 с min 480x320, при этом файл большой и содержит много длинных текстов.
   - ScrollView есть, но стоит руками проверить все вкладки на 125-150% DPI, особенно AI/STT/Profile.

## Техдолг и миграционный шум

1. В Slint/Rust glue много комментариев вида `Phase E6`, `Phase B2`, `React/Tauri`, `src-tauri`.
   - Примеры: `ui/replay.slint:3`, `src/slint_events.rs:3`, `src/bin/overlay_host.rs:1534`, `src/replay_backend.rs:3`.
   - Это не runtime-баг, но мешает быстро понять, что актуально, а что история миграции.

2. `src/bin/overlay_host.rs` очень большой: около 5179 строк.
   - Там смешаны window spawning, hotkeys, settings handlers, AI streaming, local AI installer, update UI, drag logic, tile management.
   - Риск: новые изменения легко ломают соседние сценарии. Хороший кандидат на постепенное выделение модулей: palette, settings handlers, tile spawning, hotkeys, update.

3. `ui/palette.slint:1` все еще называется `F4 KB palette stub`.
   - Но `src/bin/overlay_host.rs:3614` уже пишет, что palette wired to real `kb::search`.
   - Документационный шум: файл выглядит как stub, хотя часть функциональности реальная.

4. `src/markdown.rs:279` содержит `sample_tile_markdown`, а `src/bin/overlay_host.rs:2031` использует его как fallback content.
   - Для dev/demo это удобно, но в production-потоке может выглядеть как фантомная функциональность.

5. `src/win32.rs:72` разделяет `make_transparent_overlay` и `make_transparent_tile`; комментарии показывают, что Slint/Win32 transparency уже ломала клики.
   - Это место хрупкое по определению: после обновления Slint/winit/Windows стоит обязательно smoke-test clickability, drag, stealth и always-on-top.

## Что я бы проверил первым руками

1. F4 palette: открыть, набрать запрос, Enter, Up/Down, Esc.
2. Settings -> STT -> Local Whisper: сохранить bearer, закрыть/открыть Settings, посмотреть не светится ли сохраненный bearer в input.
3. Tile markdown: длинная shell-команда, YAML/JSON блок, таблица шире тайла.
4. `+ tile` без `SLINT_OVERLAY_DEMO`: понять, это ожидаемый fallback или недоделанная production-кнопка.
5. UI scaling: 125% и 150% DPI, русский интерфейс, узкий экран.

## Короткий вывод

Slint-часть выглядит не сырой: видно, что уже исправляли реальные проблемы с `TouchArea`, click-through, HiDPI и тайлами. Но остались несколько конкретных мест, где UI обещает больше, чем реально делает: особенно F4 Enter/keyboard navigation, `+ tile` fallback/demo path и inconsistent secret handling для Whisper bearer.

