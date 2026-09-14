# Differential security review: Windows audio settings

Date: 2026-09-14. Coordinator review, not independent model acceptance.
Scope baseline: `2ab0dee01dda8632830aee1792543bc0d434237b`.
Implementation candidate reviewed: `749813a8ed368d2585b7fc86be9e6636d928bce5`.
The final delta from 7169b687 only corrects test-fixture initialization order;
no production security boundary changed. The review includes per-endpoint error propagation and the feature-gated,
ignored MCP test fixture. The fixture does not call production startup or load
or save configuration; synthetic names and save failures are test-only.

## Scope and evidence

Reviewed `settings_audio.rs`, its Settings controller/UI wiring, translation
copy, `audio::list_devices`, and the existing `config::save` implementation.
Checked the full task diff and new files. Traced the existing session, PTT,
mic-test and wizard readers of the unchanged device configuration fields.

`git log -S` and blame identify the previous microphone setter in `7477fe1f`,
the atomic-write logic in `30b981b2f`, and secret-redacted backups in `4941c79b6`.
The patch reuses those atomic-save and redacted-backup paths without changing
credentials or serialization. The read-only whitespace check passed.
Native regression-test results are recorded separately in the audit README.

## Findings

No confirmed security regression was found in this scope. This is not a claim
that the application or dependencies are secure.

- OS device names enter a Slint string model, then an index-validated callback.
  Index zero maps to `None` independently of the translated label. Negative,
  out-of-range and unavailable-row indices cannot produce a persisted value.
- The config writer clones the current snapshot under its existing write lock,
  changes only one audio field, invokes the existing save, and installs the
  new in-memory snapshot only on success. Other settings and credentials are
  preserved. Config error chains are discarded before UI output.
- Device names never enter shell commands, filesystem paths, network URLs or
  HTML evaluation. Local driver/device-name control provides no new command
  execution path. Names remain visible as in the existing microphone selector.
- No credentials, private transcripts, provider changes, dependencies or network
  operations were added. New UI errors contain only fixed translated text.
- Missing explicit devices remain explicit: the fix does not silently capture
  another input or reroute TTS/system playback.

## Coverage boundaries

No live credentials or user config were printed or used for security tests.
Tests use synthetic in-memory config and a simulated save failure. Real
filesystem-denial/disk-full fault injection and independent model review were
not performed. This review does not audit all existing config writers, native
drivers, WASAPI recovery, Slint internals, or repository dependencies.
The existing friendly-name ambiguity for two identically named devices remains;
changing the persistence schema to endpoint IDs is outside the approved scope.
