# Original settings C05/C06: bounded in-memory mutation and optimistic UI evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_settings_mutate_rollback_confirmed.py) inspect frozen Settings tab event callbacks and configuration persistence logic. They do not spawn UI event loops or write live configurations to disk. C05 and C06 remain confirmed mechanisms.

## C05 — in-memory configuration mutation without rollback on disk save failure

In `settings_ai.rs`, [on_ai_provider_changed](<../../../slint-experiment/src/bin/overlay_host/settings_ai.rs#L745-L815>) mutates fields directly in `let mut c = cfg_c.write()`:
`c.ai_provider = provider.to_string();`
If `overlay_backend::config::save(&c)` fails, the function logs an error and returns immediately without rolling back `c.ai_provider` or any accompanying provider changes to their previous states.
In contrast, [update_cloud_model](<../../../slint-experiment/src/bin/overlay_host/settings_stt.rs#L92-L105>) in `settings_stt.rs` demonstrates an explicit rollback pattern:
`let previous = std::mem::replace(&mut config.stt_model, next.to_string()); if let Err(err) = save(config) { config.stt_model = previous; return Err(err); }`.

## C06 — optimistic live UI and runtime state changes preceding disk save

In `settings_controller.rs`:
- [on_language_selected](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L560-L609>) invokes `slint::select_bundled_translation(lang)` to switch UI translation tables live before attempting `overlay_backend::config::save(&c)`;
- [on_tile_monitor_changed](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L527-L547>) updates runtime pin state via `set_global_tile_monitor(pin)` before saving configuration;
- [on_stealth_changed](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L423-L452>) updates process-wide stealth and calls `registry_stealth.apply_stealth(on)` regardless of whether `config::save(&c)` succeeds.

## Limits

No simulated disk write failures were injected into live running instances, and no multi-threaded UI state desynchronization was tested live. Original statuses in `candidates.json` remain `confirmed`.
