# План: Пакет исправлений настроек и транзакций конфигурации (DEF-12..DEF-17, P1)
Ветка: `codex/fix-settings-transactions`
Цель: Устранить дефекты P1 в управлении конфигурацией, импорте профилей и состоянии UI настроек:

## Что делаем:
1. **DEF-12 (`overlay-backend/src/config.rs`):**
   - В `import_server_settings_from` использовать `apply_server_settings` вместо `merge_server_settings`, чтобы локальный путь `stt_gigaam_dir` текущего ПК не перезаписывался чужим значением из файла.
2. **DEF-13 (`slint-experiment/src/bin/overlay_host/settings_controller.rs`):**
   - В `on_import_profile_clicked` сохранять локальные аппаратные привязки текущего ПК (`stt_gigaam_dir`, `mic_device`, `system_audio_device`) при импорте внешнего профиля.
3. **DEF-14 (`slint-experiment/src/bin/overlay_host/settings_ai.rs`):**
   - В `on_ai_bearer_save` и `on_groq_api_key_save` разрешить очистку токенов при передаче пустой строки (удаление ключа из конфигурации вместо пропуска сохранения).
4. **DEF-15 (`slint-experiment/src/bin/overlay_host/settings_ai.rs`):**
   - В `on_ai_provider_changed` добавить откат `*c = previous` в оперативной памяти при ошибке сохранения `config::save(&c)`.
5. **DEF-17 (`slint-experiment/src/bin/overlay_host/settings_controller.rs`):**
   - Вынести инициализацию переключателей коучинга, авто-тайлов, подавления тайлов и ретеншна в функцию `reseed_persistent_controls` и вызывать её как при первом открытии, так и при повторном использовании существующего окна и после импорта профиля.

## Как проверяем:
1. `git diff --check` на отсутствие форматировочных ошибок.
2. Добавление unit-тестов в `overlay-backend` и `slint-experiment`.
3. Прогон тестов на воркере `windows-worker` по точному SHA.
4. Обновление статусов в `DEFECTS.md`.
