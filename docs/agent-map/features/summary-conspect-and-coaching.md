# Summary, resumable conspect and coaching: source-linked contract

**Evidence:** source inspection at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; no provider/model inference, live coaching measurement, native Slint or sidecar fault test executed. Live summary, saved recap, re-summary, tile regenerate, debrief and live-coaching style are distinct behaviors.

## Summary source and user entry

[Bar summary](../../../slint-experiment/src/bin/overlay_host_windows.rs#L3580-L3677) snapshots full transcript/session ID and overflow flag, rejects empty input and uses UI busy latch plus worker runtime. Full accumulator may already have discarded earlier lines; “full” means retained source, not proof an arbitrarily long day remains complete.

[run_meeting_summary](../../../overlay-backend/src/runtime.rs#L245-L376) resolves `ai_endpoint(true)`, language, monitor/stealth, formats both sources and hashes transcript. If `force=false` and saved fingerprint matches, returns saved final recap or resumes saved parts. Profile, model, memory or prompt changes are not necessarily represented by the transcript-only fingerprint; cached recap reuse is not automatic regeneration with new settings.

[Archive sources](archive-playback-and-retranscription.md) may supply catalog/prompt fallback or two channel-aggregated re-STT blocks. Summary receives text and labels, not a certified speaker identity map. [summary prompt](../../../overlay-backend/src/runtime/summary_plan.rs#L182-L257) instructs facts/decisions/action items and admits multiple interlocutors/uncertain attribution; these instructions are not model factuality proof.

## Direct versus map/reduce

[Budget plan](../../../overlay-backend/src/runtime/summary_plan.rs#L4-L128) uses output 8192/partial 2048 tokens, cloud input char heuristic 24000 and local chunk chars 20000. [Direct fit](../../../overlay-backend/src/runtime.rs#L553-L572) tries exact managed local tokenize count when context ceiling applies and falls back to conservative heuristic; non-managed cloud route with no context ceiling simply passes fit. Do not call every provider context exactly counted.

[Fresh conspect construction](../../../overlay-backend/src/runtime.rs#L325-L376) records sources before mapping. [ensure_map_parts_fit](../../../overlay-backend/src/runtime.rs#L617-L677) recursively splits source chunks until requests fit; [map loop](../../../overlay-backend/src/runtime.rs#L883-L980) fills only missing parts and never intentionally reduces incomplete map. Completed parts are saved best effort after each successful call. [reduce loop](../../../overlay-backend/src/runtime.rs#L679-L807) packs partial summaries, performs intermediate reductions and fails when no progress or single part cannot fit.

[Final path](../../../overlay-backend/src/runtime.rs#L988-L1074) sanitizes result, persists final_summary best effort, deletes forced-run backup on success and journals summary if a journal can be opened. Failure emits generic error tile with session ID and may restore backup. UI tile success does not prove every sidecar/journal write succeeded.

## Persistence and retry

[Conspect model](../../../overlay-backend/src/conspect.rs#L43-L142) stores original source slices, optional summaries, single-pass flag and final recap. [Safe file name/save](../../../overlay-backend/src/conspect.rs#L158-L225) uses temp file then plain rename; errors are logged/bool false and caller can continue in memory. No fsync or authoritative crash/power-loss durable-write success is established. Fixed same-session temp path/concurrent calls and replacement semantics need platform tests.

[Forced-run backup/restore](../../../overlay-backend/src/conspect.rs#L450-L498) moves old .json to .bak and attempts restoration if build fails. Backup/restore/drop routines are best effort; ignored failed backup means previous good recap not guaranteed retained. [Retention](../../../overlay-backend/src/conspect.rs#L291-L346) keeps newest 500 matching files independently of session retention. [retry_meeting_summary](../../../overlay-backend/src/runtime.rs#L377-L437) loads saved artifact; if sources unavailable emits error rather than requiring fresh audio by default.

[Host error retry](../../../slint-experiment/src/bin/overlay_host_windows.rs#L2281-L2337) uses summary busy UI state and worker task. [Summary tile dialogue seed](../../../slint-experiment/src/bin/overlay_host_windows.rs#L2094-L2166) prefers persisted source parts, else live transcript; generic regenerate/follow-up is not replay of original N-step map/reduce. Missing seed disables rebuild; a retained recap alone is not enough evidence to reconstruct original entire source.

## Managed prep and protocol scope

[ManagedPrepSession](../../../overlay-backend/src/runtime.rs#L439-L513) acquires both HTTP AI permits and destructive local lifecycle lease only for managed local route, resolves context/model and restores selected live model afterward. [finish caller](../../../overlay-backend/src/runtime.rs#L813-L850) sets exclusive flag from whether managed prep session exists. [summary_complete](../../../overlay-backend/src/runtime.rs#L574-L599) uses exclusive local compatibility call only on that flag; otherwise endpoint-aware native protocol completion. Codex routing is not inevitably broken by helper branch merely dropping protocol when exclusive.

Prep restore/cancellation, block-on model switch and saved config route changes while summary is in flight remain lifecycle hypotheses. Process/memory policy is described in [managed local contract](managed-local-ai.md), not verified by this recap's existence.

## Coaching modes

[Post-meeting debrief gate](../../../slint-experiment/src/slint_session.rs#L1550-L1654) requires explicit opt-in, active configured provider, minimum 30-second session and at least five mic lines. [Debrief request](../../../overlay-backend/src/runtime.rs#L75-L161) uses mic-only snapshot and active prep endpoint, requests three observations and saves text best effort. Source comment claiming only 80 lines arrives conflicts with actual stopped full_transcript snapshot; do not assume strict debrief input/token bound from that comment.

[Error notice](../../../overlay-backend/src/runtime.rs#L136-L152) is generic/deep-lock localized tile; [saved debrief](../../../overlay-backend/src/conspect.rs#L356-L411) allows archive read-only reopen. It is not silent log-and-drop anymore despite older rustdoc. Generated language follows response_language, UI chrome ui_language.

[Live coaching style](../../../overlay-backend/src/runtime/trigger_detect.rs#L25-L62) adds read-aloud wording constraints to auto-tile prompt, controlled by config Settings toggle. It does not establish a numeric WPM/filler-density rolling 60-second pill: reviewed source/UI search found no active WPM metric/setter surface. Earlier broad Voice Coach live-pill claims remain unverified/historical, not inferred from a prompt-style switch.

## Verification boundaries

[Summary pure tests](../../../overlay-backend/src/runtime/tests.rs) and [conspect tests](../../../overlay-backend/src/conspect.rs#L500-L761) declare reusable parts/retry/save/backup/format behavior but were not Cargo-executed. Exact-SHA acceptance needs long context fit, forced/nonforced settings-change reuse, missing-part retry, disk/rename failure, summary-debrief mutual pressure, managed prep restore, archive same-session follow-up and correct status on success/failure. Prompt promises and stored artifact presence are not proof of faithful content or complete durability.
