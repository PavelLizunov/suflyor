# Original settings C01/C08: bounded window reuse and full-profile import evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Seven [fixtures](../operations/test_settings_reuse_profile_import_confirmed.py) inspect frozen Settings window reuse, control reseeding, and profile import logic in `settings_controller.rs` and `settings_import_export.rs`. They do not open native GUI windows, launch file dialogs, or overwrite user configurations. C01 and C08 remain confirmed mechanisms.

## C01 — Reused Settings window skips persistent control reseeding

In `settings_controller.rs`:
- When reopening an existing Settings window ([L106-L137](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L106-L137>)), `open_settings` executes `populate_token_status`, `refresh_profiles`, and `populate_component_rows`, but completely skips the persistent toggle seeding block that runs on initial construction;
- Specifically, properties such as `set_coaching_debrief`, `set_coaching_live_tiles`, `set_record_audio`, `set_auto_tiles_enabled`, `set_suppress_tiles`, `set_trigger_keywords_input`, `set_retention_mode`, and `set_retention_value` ([L174-L196](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L174-L196>)) are only seeded on fresh window creation;
- When a user imports a profile via [on_import_profile_clicked](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L700-L735>), it calls [msg_refresh_after_import](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L1300-L1306>), which only invokes `populate_token_status`, leaving the visible toggles unrefreshed until application restart.

## C08 — Full-profile import replaces live config without host path allowlisting

In `settings_controller.rs`:
- [on_import_profile_clicked](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L700-L735>) assigns `*cfg_c.write() = imported;`, replacing the entire live configuration without filtering machine-local paths (such as `stt_gigaam_dir`, `ai_local_base_url`, `mic_device`, or audio endpoints);
- In contrast to server import (which explicitly preserves the local PC's GigaAM path), full profile import overwrites local paths with foreign machine values;
- Export success messages format `path.display()` directly ([settings_controller.rs#L684](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L684>) and [settings_import_export.rs#L72](<../../../slint-experiment/src/bin/overlay_host/settings_import_export.rs#L72>)), exposing local file system directory structures;
- [populate_token_status](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L1906>) populates `win.set_stt_whisper_bearer_input` with the raw saved token string, whereas cloud AI and OpenAI/Anthropic keys are kept blanked.

## Limits

No user configuration files were overwritten, no native file dialogs were triggered, and no GUI window state was inspected live. Original statuses in `candidates.json` remain `confirmed`.
