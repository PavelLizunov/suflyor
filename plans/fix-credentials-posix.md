# План: Исправление хранилища учетных данных POSIX (DEF-03, DEF-04)
Ветка: `codex/fix-credentials-posix`
Цель: Устранить дефекты DEF-03 и DEF-04 в `overlay-backend/src/credentials.rs`:
1. DEF-04: `credentials_path()` безусловно создавал папку `suflyor/` через `dirs::config_dir()?.join("suflyor")`, отсекая существующую папку данных `overlay-mvp/`. Заменяем на `crate::paths::data_root()`.
2. DEF-03: `read_map()` на POSIX при повреждении JSON возвращал пустую карту `HashMap::new()`, что при последующей записи или удалении одного слота молча уничтожало все остальные сохранённые ключи в файле. Заменяем на явный `Result<HashMap<String, String>>`.

## Как проверяем:
1. `git diff --check` на отсутствие форматировочных ошибок.
2. Добавление unit-теста `posix_credentials_preserves_corrupt_file_on_read_failure`.
3. Запуск тестов на `windows-worker` и `mac-worker` по точному SHA.
4. Обновление статусов DEF-03 и DEF-04 в `DEFECTS.md`.
