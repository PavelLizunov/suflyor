# Original bridge C07 / settings C09: bounded UI thread blocking and error leakage evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_ui_blocking_and_error_leak_hypotheses.py) inspect frozen session orchestration and settings connection test logic. They do not block UI threads, capture audio, or make network calls. C07 and C09 remain hypotheses.

## C07 — synchronous blocking operations on the session control thread

In `slint_session.rs`, [start_session_inner](<../../../slint-experiment/src/slint_session.rs#L273-L490>) performs multiple synchronous, potentially slow I/O and validation calls inline on the calling thread:
- `stt::validate_gigaam_dir(model_dir)` validates on-disk model directories (~0.5s);
- `audio::start_capture(mic_dev, sys_dev)` initializes WASAPI audio capture devices;
- `crate::journal::open_session_id` opens the JSONL journal file.

In [stop_session](<../../../slint-experiment/src/slint_session.rs#L1396-L1480>), the calling thread executes:
`if let Err(e) = j.shutdown(Duration::from_secs(3))`
synchronously blocking up to 3 seconds waiting for journal flush durability before returning or spawning catalog re-indexing.

## C09 — raw error chain formatting and length truncation in connection test callbacks

In `settings_ai.rs`, [on_ai_bridge_test_clicked](<../../../slint-experiment/src/bin/overlay_host/settings_ai.rs#L1380-L1405>) catches provider errors and formats the entire chain via `format!("[err] {e:#}").chars().take(90).collect()`.
In `settings_stt.rs`, [on_stt_test_clicked](<../../../slint-experiment/src/bin/overlay_host/settings_stt.rs#L209-L235>) similarly captures STT connection errors with `format!("[err] {e:#}").chars().take(90).collect()`.
In `settings_controller.rs`, [on_mic_test_clicked](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L375-L395>) formats microphone capture errors directly as `format!("error: {e}")`.
While error messages are truncated to 90 characters, raw error chains containing server hostnames, ports, or protocol errors are exposed to the UI rather than mapped to sanitized generic failure codes.

## Limits

No UI thread freeze or priority inversion was measured with timing profilers, no live network tests were triggered, and no sensitive credentials were leaked in error logs. Original statuses in `candidates.json` remain `hypothesis`.
