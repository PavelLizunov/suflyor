# Original settings C02/C07: bounded install flag resets and property seeding evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_settings_reset_seeding_hypotheses.py) inspect frozen Settings controller state initialization and property seeding logic. They do not run UI event loops, background installation tasks, or thread pools. C02 and C07 remain hypotheses.

## C02 — unconditional installer flag resets on settings window reuse

In `settings_controller.rs`, [populate_token_status](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L1604-L1920>) is invoked every time the Settings window is opened or reused.
It unconditionally clears component and engine installer progress flags:
- `win.set_ocr_installing(false)`
- `win.set_diar_installing(false)`
- `win.set_update_checking(false)`
- `win.set_tts_installing(false)`
- `win.set_tera_installing(false)`
- [reset_component_install_state](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L1462-L1466>): resets `component_busy_index` to `-1` and phase to `0`.

The GigaAM installer is the sole exception, guarded by:
`if !win.get_stt_gigaam_installing() { win.set_stt_gigaam_install_failed(false); }`.

## C07 — unseeded properties relying on Slint component defaults

In `populate_token_status`, several properties modified by tab-specific event handlers are omitted from the initial seed walk:
- `ai_local_model_profile_index` and `ai_local_vision_available` are written in `settings_local_ai.rs` and `settings_ai.rs`, but are not seeded during `populate_token_status`, inheriting Slint defaults (`0` and `false`) on window construction.
- `win.set_server_preview_ready(false)` disables preview actions on open, but the associated `server_preview_*` text strings are not blanked.
- `context_processing` and `context_dictating` flags are not explicitly cleared in `populate_token_status`.

## Limits

No long-running background installer tasks were cancelled or orphaned, no UI glitch was visually inspected, and no state machine race was executed. Original statuses in `candidates.json` remain `hypothesis`.
