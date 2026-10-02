# Original config C08/C09: bounded server settings merge and load/save error logging evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_config_merge_logging_confirmed.py) inspect frozen configuration import, merge, and error logging logic. They do not overwrite user configurations, write filesystem files, or log live usernames. C08 and C09 remain confirmed mechanisms.

## C08 — `import_server_settings_from` clobbers machine-local `stt_gigaam_dir`

In `config.rs`, [merge_server_settings](<../../../overlay-backend/src/config.rs#L1696-L1750>) copies `imported.stt_gigaam_dir` directly onto `current.stt_gigaam_dir`.
In [import_server_settings_from](<../../../overlay-backend/src/config.rs#L1765-L1775>), it calls `merge_server_settings` and directly persists the result via `save(&next)`, overwriting the machine-local GigaAM model directory path.
In contrast, the UI's documented Apply path, [apply_server_settings](<../../../overlay-backend/src/config.rs#L2052-L2058>), explicitly re-asserts the local machine's directory:
`next.stt_gigaam_dir = current.stt_gigaam_dir.clone();`.

## C09 — filesystem path formatting in load/save errors and default write on corrupt load

In `config.rs`, [config_path](<../../../overlay-backend/src/config.rs#L1353-L1359>) formats error messages containing `dir.display()` (`format!("create config dir {}", dir.display())`), which on standard systems incorporates the local OS username (`%APPDATA%\...` or `/home/<user>/...`).
When loading configuration in [load](<../../../overlay-backend/src/config.rs#L1410-L1515>), if JSON parsing fails, it calls `preserve_corrupt_config` and falls back to `Config::defaults()`. If subsequent migration routines set the `dirty` flag (such as legacy TTS migration or schema version bumping), `save(&cfg)` executes, writing defaults over the user's configuration file.

## Limits

No user configuration files were overwritten, no external settings files were imported, and no OS paths were captured in live telemetry. Original statuses in `candidates.json` remain `confirmed`.
