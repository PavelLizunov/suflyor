# Settings and portable configuration: source-linked feature contract

**Evidence:** source inspection at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; no live config file, credentials, native Settings UI or filesystem fault test executed. This is owning-controller/save/import/reset behavior, not the full configuration key schema.

## Local state and persistence

[Config path](../../../overlay-backend/src/config.rs#L1353-L1380) resolves product data-root JSON and UTF-8/BOM parsing. [Data-root policy](../../../overlay-backend/src/paths.rs#L22-L40) prefers current product dir and falls back to legacy only if current does not exist; [migration](../../../overlay-backend/src/paths.rs#L60-L98) attempts one directory rename before other startup touches. An unrelated helper creating an empty new dir can alter path selection; do not infer both locations are merged.

[load](../../../overlay-backend/src/config.rs#L1402-L1503) parses defaults/migrations, repairs recognized mojibake and stamps schema version. [Corrupt preservation](../../../overlay-backend/src/config.rs#L1383-L1400) tries rename-to-broken then falls back to defaults even when preservation fails; source comment or success-style log wording is not a guaranteed recoverable backup.

[save_to_path](../../../overlay-backend/src/config.rs#L1561-L1639) writes sibling temp, applies POSIX file permissions, creates a redacted old-config backup and renames into place. Rename failure fallback removes old target before retry; absent explicit fsync and multi-call same-temp path means blanket crash/power-loss atomic durability is unverified. A returned Err after live config mutation can leave runtime/disk diverged.

[SharedConfig](../../../overlay-backend/src/config.rs#L2063-L2070) is an Arc/RwLock value. Saved Config includes legacy Groq/bridge/local/server bearers; direct provider keys use [separate credentials](../../../overlay-backend/src/credentials.rs#L1-L26). [POSIX credentials file](../../../overlay-backend/src/credentials.rs#L133-L217) is restricted by mode, not encryption. Exports/backups are sensitive even when some credential fields are redacted, because meeting notes/profiles remain private.

## Settings lifecycle

[open_settings](../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L104-L145) has a reuse branch when slot exists; it refreshes tokens/profiles/devices/component rows and selected runtime state. [Explicit close](../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L302-L312) sets slot None, so not every open reuses a process-lifetime singleton.

[populate_token_status](../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L1604-L1979) blanks/reseeds transient statuses and route/engine controls. [Guard test](../../../slint-experiment/tests/settings_reset_guard.rs#L20-L26) deliberately covers status/result strings plus named update results, **not every busy flag/state machine**. Setter presence is not functional acceptance or proof that asynchronous older completions cannot overwrite current choice.

[Provider save handlers](../../../slint-experiment/src/bin/overlay_host/settings_ai.rs#L727-L832) mutate config then save with heterogeneous rollback behavior. [STT cloud-model helper](../../../slint-experiment/src/bin/overlay_host/settings_stt.rs#L14-L29) explicitly rolls back on failed save; this is a counterexample, not assurance for every controller. [Language/monitor/stealth callbacks](../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L423-L447) and [language/monitor](../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L527-L582) can apply runtime intent before persistence; security-intent failure UX must be chosen deliberately.

## Full profile transfer

[Full export](../../../overlay-backend/src/config.rs#L1645-L1662) serializes portable Config including transferable legacy keys/notes; deep_lock is machine-local and omitted. [Full import](../../../overlay-backend/src/config.rs#L1664-L1687) parses/saves replacement while preserving local deep-lock policy. It is intentionally more destructive to local settings than server-only import, not an unauthorized merge.

[UI full import](../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L695-L728) swaps shared config only after backend save succeeds, populates token/status fields and shows restart-needed label. Other already-live processes/UI/profile lists may not immediately mirror every replaced field; explicit restart advice does not prove unnoticeable stale state is acceptable. Export success/error UI includes selected path and may require redaction for screenshot-sharing policy.

## Server-only preview/apply

[merge_server_settings](../../../overlay-backend/src/config.rs#L1696-L1755) copies an explicit field set (AI/vision/STT endpoint/model/auth knobs) onto current config, preserving local notes/theme/hotkeys/deep lock and selected empty legacy Codex choice rules. It copies some nominal STT-language/machine path fields; field ownership comes from actual assignments, not function name alone.

[UI preview](../../../slint-experiment/src/bin/overlay_host/settings_import_export.rs#L86-L120) loads user file, stores parsed pending config, displays old/new groups and credential presence, not credential values. [Apply](../../../slint-experiment/src/bin/overlay_host/settings_import_export.rs#L126-L184) requires preview-ready and pending object, calls [apply_server_settings](../../../overlay-backend/src/config.rs#L2045-L2056) to preserve this PC's GigaAM path, saves next config **before** replacing live value, then refreshes UI. Compare backend [import_server_settings_from](../../../overlay-backend/src/config.rs#L1758-L1770), which uses merge directly and does not apply that UI-specific local-path preservation.

[Server export](../../../overlay-backend/src/config.rs#L1777-L1797) overlays server fields on defaults then writes pretty portable JSON; it intentionally contains legacy server credentials. Direct provider/Codex secret storage is not simply bundled into this file.

[Preview host masking](../../../overlay-backend/src/config.rs#L1862-L1899) is not a general URL/secret redactor: it retains suffix path/query/fragment, and malformed input requires care. [Preview composer](../../../slint-experiment/src/bin/overlay_host/settings_import_export.rs#L188-L238) also redacts local-home text. Structured `has_key` booleans do not establish every URL/model/path string is screenshot-safe.

## Verification boundaries

[Config tests](../../../overlay-backend/src/config/tests.rs) and [Settings reset guard](../../../slint-experiment/tests/settings_reset_guard.rs) declare path/merge/reset/permissions behavior. Native guard execution, re-open mid-download, stale endpoint replies, failed disk save, importing older provider config, secret-safe screenshots and populated all-tab controls remain unexecuted. This research never reads a user's actual config or portable export.
