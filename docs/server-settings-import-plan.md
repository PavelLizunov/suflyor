# Server Settings Import Plan

Цель: сделать перенос настроек на другой компьютер осознанным и безопасным. Текущий full import полезен как backup, но для миграции между ПК нужен отдельный импорт только серверных AI/STT настроек, без перетирания профилей, UI, устройств и локальных предпочтений.

## Текущая проблема

Сейчас импорт настроек работает как полная замена конфига:

- `slint-experiment/ui/settings_panel.slint:692` — UI предупреждает, что импорт перезаписывает текущие настройки.
- `slint-experiment/src/bin/overlay_host.rs:4761` — handler вызывает `overlay_backend::config::import_from(&path)`.
- `overlay-backend/src/config.rs:1793` — `import_from` читает JSON, десериализует весь `Config` и сохраняет его целиком.

Это неудобно для переноса на другой компьютер: вместе с серверными настройками можно случайно перетереть локальные вещи вроде audio device names, monitor name, профилей, темы, hotkeys и snippets.

## Предлагаемый UX

Оставить текущие кнопки full backup/import как есть, но рядом добавить отдельный сценарий:

- `Export settings (incl. keys)...` — полный backup, как сейчас.
- `Import settings...` — полный restore, как сейчас.
- `Import server settings...` — новый безопасный режим переноса AI/STT серверов.

Текст рядом с кнопками должен явно различать режимы:

```text
Full import overwrites the whole config.
Server import only copies AI/STT providers, URLs, models and keys; local UI, profiles, devices and snippets stay unchanged.
```

## Что импортировать в server-only режиме

Импортировать только поля, которые описывают AI/STT серверы, модели, провайдеры и ключи:

```text
ai_provider
ai_base_url
ai_bearer
ai_model
prep_model
ai_prompt_cache

ai_local_base_url
ai_local_bearer
ai_local_model
ai_local_prep_model
ai_local_vision
ai_local_thinking

stt_provider
groq_api_key
stt_language
stt_model

stt_gigaam_dir
stt_gigaam_gpu

stt_whisper_url
stt_whisper_bearer
stt_whisper_model
```

## Что НЕ импортировать

Эти поля должны остаться текущими на новом компьютере:

```text
meeting_context
context_profiles
active_profile

mic_device
system_audio_device
tile_monitor_name

trigger_keywords
auto_tiles_enabled
detector_skip_mic
auto_tile_every_line

stealth_enabled
post_meeting_debrief_enabled
custom_css
auto_export_on_quit
max_session_cost_usd

hotkey_ask
hotkey_screenshot
hotkey_toggle_visibility
hotkey_pause_audio
manual_ask_mode

ui_language
color_scheme
tile_font_size
tile_body_opacity

snippets
```

Причина: это локальные, UX или пользовательские данные, которые либо завязаны на конкретный ПК, либо не относятся к серверному подключению.

## Backend implementation sketch

Добавить в `overlay-backend/src/config.rs` отдельную функцию:

```rust
pub fn import_server_settings_from(path: &std::path::Path, current: &Config) -> Result<Config> {
    let bytes = std::fs::read(path).context("read server settings import file")?;
    let imported: Config = serde_json::from_slice(&bytes).context("parse server settings JSON")?;

    let mut next = current.clone();

    next.ai_provider = imported.ai_provider;
    next.ai_base_url = imported.ai_base_url;
    next.ai_bearer = imported.ai_bearer;
    next.ai_model = imported.ai_model;
    next.prep_model = imported.prep_model;
    next.ai_prompt_cache = imported.ai_prompt_cache;

    next.ai_local_base_url = imported.ai_local_base_url;
    next.ai_local_bearer = imported.ai_local_bearer;
    next.ai_local_model = imported.ai_local_model;
    next.ai_local_prep_model = imported.ai_local_prep_model;
    next.ai_local_vision = imported.ai_local_vision;
    next.ai_local_thinking = imported.ai_local_thinking;

    next.stt_provider = imported.stt_provider;
    next.groq_api_key = imported.groq_api_key;
    next.stt_language = imported.stt_language;
    next.stt_model = imported.stt_model;

    next.stt_gigaam_dir = imported.stt_gigaam_dir;
    next.stt_gigaam_gpu = imported.stt_gigaam_gpu;

    next.stt_whisper_url = imported.stt_whisper_url;
    next.stt_whisper_bearer = imported.stt_whisper_bearer;
    next.stt_whisper_model = imported.stt_whisper_model;

    save(&next).context("persist imported server settings")?;
    Ok(next)
}
```

Почему функция принимает `current: &Config`: так она не зависит от глобального состояния и проще тестируется.

## Slint/Rust UI wiring

В `settings_panel.slint` добавить:

```text
callback import-server-settings-clicked();
```

В блоке Backup / transfer settings добавить кнопку:

```text
Import server settings...
```

В `overlay_host.rs` добавить handler рядом с текущим `on_import_profile_clicked`:

1. Открыть `rfd::FileDialog`.
2. Взять текущий config snapshot.
3. Вызвать `config::import_server_settings_from(&path, &snapshot)`.
4. Записать результат в shared config: `*cfg_c.write() = imported;`.
5. Обновить UI через существующий refresh-путь (`msg_refresh_after_import` или отдельный server-import refresh).

Сообщение пользователю после успеха:

```text
[ok] server settings imported: AI/STT providers, URLs, models and keys. Profiles, devices, UI and snippets were kept.
```

Если импортированы локальные пути:

```text
[ok] server settings imported. Check local model paths on this PC: GigaAM/Whisper/llama locations may differ.
```

## Tests to add

В `overlay-backend/src/config.rs` добавить unit test:

1. `current` содержит уникальные профили, audio device names, UI language, snippets, hotkeys.
2. `imported` содержит другие AI/STT настройки и другие локальные поля.
3. После `import_server_settings_from`:
   - AI/STT поля взяты из `imported`.
   - profiles/devices/UI/snippets/hotkeys остались из `current`.

Проверить минимум:

```text
ai_provider
ai_base_url
ai_bearer
ai_model
ai_local_base_url
ai_local_model
stt_provider
groq_api_key
stt_gigaam_dir
stt_whisper_url

meeting_context unchanged
context_profiles unchanged
mic_device unchanged
system_audio_device unchanged
ui_language unchanged
color_scheme unchanged
snippets unchanged
hotkeys unchanged
```

## Optional later improvement

Позже можно добавить не только импорт, но и отдельный экспорт:

```text
Export server settings...
```

Он будет создавать JSON только с серверными полями или full `Config` с локальными полями по умолчанию. Но для первого шага достаточно server-only import из текущего full export файла.

