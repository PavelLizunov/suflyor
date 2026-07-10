# Goal — Hermes-плагин: установка в один клик из приложения (v0.33.0)

**Дата:** 2026-07-10 · **Релиз:** v0.33.0 (@09f4c67, Latest) · **Статус:** SHIPPED, у тестера

## Зачем

Владелец отклонил поток v0.32.0 «скачай zip → распакуй → запусти install.ps1»:

> «запуск и установка всего должно проходить исключительно из приложения,
> а не через сторонние штуки — это противоречит базовым принципам»

Прецедент в приложении уже есть: TTS/OCR-движки ставятся кнопками из
Настроек. Плагин Hermes обязан ставиться так же. Попутно вскрылось, что
`install.ps1` целил в `~/.hermes` — на Windows домашний каталог Hermes на
самом деле `%LOCALAPPDATA%\hermes` (проверено по исходникам hermes-agent,
`hermes_constants.get_hermes_home()`), т.е. скрипт был ещё и некорректен.

## Что сделано

- **`overlay-backend/src/hermes_install.rs`** — исходники плагина зашиты в
  бинарь (`include_str!`). `install_plugin(url, token)`:
  1. пишет `plugin.yaml` + `__init__.py` в `<hermes home>/plugins/suflyor/`
     (перезапись = обновление, идемпотентно);
  2. line-merge `SUFLYOR_BRIDGE_URL`/`SUFLYOR_BRIDGE_TOKEN` в `.env` —
     остальные строки байт-в-байт; **нечитаемый `.env` = стоп с ошибкой**,
     а не перезапись файла секретов (баг пойман на self-review);
  3. включает `suflyor` в `plugins.enabled` конфига **точечной текстовой
     правкой** — комментарии пользовательского config.yaml сохраняются
     (официальный `hermes plugins enable` пере-дампливает весь YAML и
     стирает их); нестандартные формы (flow-списки) не трогаются —
     статус подсказывает выполнить CLI вручную.
- Hermes home = `HERMES_HOME` env → `%LOCALAPPDATA%\hermes` (Win) /
  `~/.hermes` (иначе) — зеркало логики hermes-agent.
- **Кнопка «Установить плагин в Hermes»** во вкладке Настройки → Hermes
  (+ статус; сброс при переоткрытии окна — settings_reset_guard). Токен
  минтится при первом использовании. URL для `.env` учитывает bind-host
  (loopback/0.0.0.0 → `127.0.0.1`, иначе сам хост).
- Удалённый Hermes (Tailscale, кейс владельца) кнопкой не покрывается
  физически — ручной путь описан в `integrations/hermes-plugin/README.md`.
- Удалено: `install.ps1`; из релиза убран ассет `suflyor-hermes-plugin.zip`.

## Верификация

- Гейт 9/9 (fmt+clippy+tests+i18n ×3 крейта) — после двух честных красных:
  fmt на новом файле и `clippy::panic` в тестах (нужен inner
  `#![allow]` — интеграционные тесты не покрываются `#[cfg(test)]`).
- 14 юнит-тестов текстовых трансформаций (.env merge: replace/append/CRLF/
  идемпотентность; config.yaml: append/insert/already/`[]`-конверсия/
  flow-Unsupported/границы блока).
- Live-смок `tests/hermes_plugin_smoke.rs` (`#[ignore]`, по требованию) на
  **копиях реальных** config.yaml (73KB) + .env владельца: дифф ровно
  +3/+4 строки в конец, повторный запуск → «уже включён». NB: имя файла
  сознательно без слова "install" — Windows UAC Installer Detection требует
  элевации для таких exe.
- Установленный билд: обе строки кнопки найдены byte-search'ем, два
  boot-смока стабильны. CI + security зелёные на 09f4c67.
- Живой скрин вкладки не снят (владелец за машиной + elevated Task Manager,
  a11y-дерево Slint пустое) → приёмка тестером: `docs/retest-hermes-v0.33.0.html`
  (шаги: установить релиз → включить мост → кнопка → перезапустить Hermes;
  новый пункт **H1b** — идемпотентность кнопки).

## Ссылки

- План интеграции + P5-раздел: `docs/goal-hermes-integration-2026-07-09.md`
- Плагин (источник embed'а): `integrations/hermes-plugin/suflyor/`
- Релиз: https://github.com/PavelLizunov/suflyor/releases/tag/v0.33.0
