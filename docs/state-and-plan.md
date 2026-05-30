# Suflyor — состояние и план (после v0.5.0, 2026-05-30)

Живой документ для продолжения работы / после компакта контекста.

## Где мы
- **Выпущен v0.5.0** (Latest) с NSIS-инсталлятором; авто-апдейтер его предлагает.
  Репо: github.com/PavelLizunov/suflyor.
- Стек: чистый Rust + Slint. Две crate-ы:
  - `slint-experiment/` — бинарь `overlay-host` (UI в `ui/*.slint`, оркестрация
    в `src/bin/overlay_host.rs`).
  - `overlay-backend/` — audio/stt/ai/runtime/config/journal/kb (без UI).
  - Сборка: `cargo build --release --bin overlay-host` (из slint-experiment).
    Инсталлятор: `scripts/build-slint-release.ps1 -Installer`.
- Юзер на **локальном AI** (gemma @ 127.0.0.1:8080) + GigaAM STT; тема **Light Frost**
  (color_scheme=3). 2 монитора: основной ландшафт 1920×1080, вторичный ПОРТРЕТ
  1200×1920 при x=-1200.

## Сделано в этой сессии (всё закоммичено + запушено, вошло в v0.5.0)
- **Бренд:** WebP→SVG (potrace) → `assets/icon.svg` (белая заливка + `colorize`
  под тему). Лого на баре 40px (accent, **тянет окно**), бледный вотермарк на фоне
  Настроек, иконка приложения (.ico/.png) обрезана до ~96% кадра (крупнее в трее/
  на ярлыке). `icon.png`/`.ico` — для трея/окна; `icon.svg` — внутри UI.
- **Slint-баги (из аудита):** F4 Enter открывает верхний результат (Up/Down
  невозможно — std LineEdit съедает стрелки); код-блоки в тайлах `no-wrap`+`clip`;
  whisper bearer очищается после Save.
- **i18n:** починена захардкоженная RU-строка плейсхолдера транскрипта (теперь
  `@tr` + запись в `ru.po`). Переключение RU/EN работает (`select_bundled_translation`
  + `ui_language` персистится).
- **«+ тайл»:** переписан — тайл спавнится **мгновенно** (прямой `TileWindow::new`)
  + реальный AI-запрос через `cfg.ai_endpoint(false)` (резолвит локал/облако).
  Пусто → тайл-подсказка; ошибка AI → тайл с ошибкой.

## План (что дальше)

### Из трёх доков (юзер просил, по порядку)
1. **#125 Импорт серверных настроек** (ФИЧА) — по `docs/server-settings-import-plan.md`:
   `config::import_server_settings_from(path, current)` копирует ТОЛЬКО AI/STT-поля
   серверов (провайдеры/URL/модели/ключи), не трогая профили/устройства/UI/хоткеи/
   сниппеты. + кнопка «Import server settings» + хендлер + юнит-тест.
2. **#126 Чистка легаси-доков** — по `docs/legacy-tauri-react-mentions.md`: убрать
   упоминания React/Tauri/WebView2 в CLAUDE.md, docs/architecture.md, ADR-001,
   security-audit, REVIEW_AGENT_PROMPT.

### Баги, которые я нашёл/пометил, но не чинил (нужно решение/фикс)
3. **Бар прыгает между основным и портретным монитором** — позиция переустанав-
   ливается по таймеру. Найти логику репозиционирования (win32.rs / placement бара)
   и зафиксировать.
4. **F6 не срабатывает** как глобальный хоткей (F4 работает, F6 — нет) **И**
   `manual_spawn_tile`/F6 всё ещё бьют в ОБЛАЧНЫЙ endpoint (сырой `c.ai_base_url`),
   а не в резолвер → перевести `manual_spawn_tile` на `cfg.ai_endpoint(false)`
   (тот же баг, что я починил в инлайне «+ тайл»).
5. **Проверить «+ тайл» вживую с активной сессией** — проверен только мгновенный
   спавн + ветка «нет транскрипта»; ветка с реальным ответом по транскрипту
   кодово верна и endpoint доступен, но вживую не подтверждена.

### Ниже приоритет (из slint-surface-audit)
6. Широкие таблицы клипаются (нет h-scroll); markdown-адаптер теряет ссылки/
   emphasis; заголовок окна Replay «(Phase 0)»; ширина бара на 125-150% DPI;
   `overlay_host.rs` ~5k строк (кандидат на разбиение).
7. **#76** Settings: Card/Row titled sections (опциональный полиш).

## Ключевые подводные камни (для будущих сессий)
- **Проверять Slint-оверлей через `CopyFromScreen` по rect HWND** (Win32
  EnumWindows+GetWindowRect по pid+title → PNG → Read). Скриншоты computer-use
  ВРУТ про цвета прозрачного оверлея (показывают тёмным на светлой теме) и не
  показывают бар, когда он на вторичном мониторе. `zoom` — в физических координатах
  экрана (1920×1080), не в координатах скриншота.
- **Бар переезжает сам / прыгает между мониторами** → синтетические MoveWindow+клики
  ненадёжны; ширина статус-пилюли сдвигает x чипов; спавн тайла rate-limited.
  Хоткеи: F4=палитра (работает), F6=ручной спавн (не срабатывает), F7=collapse-заглушка.
- **AI endpoint:** `cfg.ai_endpoint(false)` резолвит локал/облако по `ai_provider`;
  сырой `c.ai_base_url` — ВСЕГДА облако (молча висит/падает для локальных юзеров).
- **i18n:** строки в `@tr()` — это английский msgid; `translations/ru/LC_MESSAGES/
  slint-replay.po` (плоские msgid/msgstr, без msgctxt) держит русский. Захардкоженная
  кириллица обходит механизм.
- **Иконки:** potrace в `%TEMP%\potrace`; `icon-source.png` — прозрачный line-art;
  генерация через `scripts/gen_icon.ps1`; SVG в Slint работает по умолчанию — НЕ
  добавлять `svg` в features slint (ломает выбор версии).
- **Релиз:** поднять версию в `slint-experiment/Cargo.toml` + `scripts/slint-installer.nsi`
  (PRODUCT_VERSION); `build-slint-release.ps1 -Installer`; `gh release create vX.Y.Z
  <installer> --title ... --notes-file ...`.
- Не коммитить `nini-context-backup.txt` (личная подготовка юзера — держать локально).
