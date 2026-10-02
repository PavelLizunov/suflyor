# Original settings C03/C04: bounded fetch_models generation and Codex config save evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_settings_models_codex_hypotheses.py) inspect frozen settings controller and AI settings UI wiring logic. They do not spawn UI event loops, make HTTP requests to model endpoints, or modify configuration on disk. C03 and C04 remain hypotheses.

## C03 — `fetch_models` lacks generation fencing

In `settings_ai.rs`, [fetch_models](<../../../slint-experiment/src/bin/overlay_host/settings_ai.rs#L447-L503>) spawns an OS thread to query `/models`, then posts an event via `slint::invoke_from_event_loop` that directly updates `w.set_ai_models` or `w.set_ai_local_models`.
The function does not capture or check an atomic generation counter.
In contrast, [refresh_codex_account_status](<../../../slint-experiment/src/bin/overlay_host/settings_ai.rs#L203-L215>) in the same file captures `let generation = invalidate_codex_snapshot_ui();` and checks `if codex_snapshot_ui_is_current(generation)` before applying results.

## C04 — `refresh_codex_account_status` writes configuration without user gesture

In `settings_controller.rs`, [open_settings](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L87-L170>) calls `refresh_codex_account_status` both when creating a new settings window and when reusing an existing one.
Inside [refresh_codex_account_status](<../../../slint-experiment/src/bin/overlay_host/settings_ai.rs#L302-L352>), if `saved.is_empty()` or reasoning effort changes, it mutates `c.codex_model` and calls `overlay_backend::config::save(&c)`. Similarly, if `saved_vision.is_empty()`, it automatically writes `c.codex_vision_model` to disk without requiring a user click or explicit save button interaction.

## Limits

No network latency was simulated, no background HTTP calls were made, and no actual disk config modifications were executed. Original statuses in `candidates.json` remain `hypothesis`.
