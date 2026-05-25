# suflyor

Личный AI-overlay для технических собесов под Windows. Слушает звук, транскрибирует через Whisper, спрашивает Claude, показывает ответ во второстепенном окошке.

Pet project, **v0.0.1 — first try**. Под одного пользователя. Без code signing, без telemetry.

![overlay bar](docs/screenshots/overlay-bar.png)

Тонкий transparent бар сверху: статус Listening, 3 HUD-dot (audio/stt/ai), push-to-talk кнопки для mic и system audio, hotkey подсказка, шестерёнка Settings.

### F4 — Knowledge Base palette (1643 entries)

![KB palette](docs/screenshots/kb-palette.png)

Поиск по embedded базе (glossary + commands + patterns). Enter → tile с найденным контентом на второстепенном мониторе (или primary, если он один). Без AI-вызова, $0.

### Settings (⚙ или tray)

![Settings](docs/screenshots/settings.png)

Перетаскивается за заголовок «⋮⋮ Settings». ✕ Выйти — quit с подтверждением. 12 секций: профили контекста, meeting context, audio devices, AI proxy, интерфейс, stealth, coaching, auto-tiles, knowledge base, snippets, STT, hotkeys (скрол).

## Установка

1. Скачать MSI из [Releases](https://github.com/PavelLizunov/suflyor/releases)
2. Запустить — SmartScreen скажет «Unknown publisher» → More info → Run anyway
3. Создать `%APPDATA%\overlay-mvp\config.json`:

```json
{
  "groq_api_key": "gsk_…",
  "ai_bearer": "<BRIDGE_SECRET>",
  "ai_base_url": "http://127.0.0.1:18902/v1",
  "ai_model": "claude-haiku-4-5",
  "prep_model": "claude-sonnet-4-6",
  "stt_model": "whisper-large-v3",
  "response_language": "ru",
  "mic_device": null,
  "system_audio_device": null
}
```

4. Запустить «suflyor» из Start Menu. Подкрутить устройства через Settings (⚙).

## Hotkeys

| | |
|---|---|
| F3 | Reask последнего вопроса |
| F4 | KB palette (поиск 1643 entries) |
| F6 | Manual tile из последней реплики |
| F8 | Pause/resume сессии |
| F9 | Ask AI |
| F10 | Screenshot для следующего ask |
| F11 | **PANIC HIDE** — скрыть overlay+tiles |

## Stack

Tauri 2 + React 19 + Rust. Groq Whisper Large v3 для STT. Claude через OpenAI-compat OAuth-bridge. WASAPI loopback для системного звука.

## Лицензия

GPL-3.0
