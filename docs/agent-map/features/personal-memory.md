# Personal memory: source-linked consent and retrieval contract

**Evidence:** source inspection at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; no cloud normalization turn, native UI action or production database read executed. This maps implemented memory behavior and explicitly separates unused/proposed normalization/vector phases from reachable paths.

## Persisted entities and consent

[Curated schema](../../../overlay-backend/migrations/0003_memory.sql) separates pending/rejected/approved candidates from active/archived memory items. [V2 columns](../../../overlay-backend/migrations/0005_memory_v2.sql) store source_text/entity/norm_status, while embedding_status remains a placeholder used as `none`; existence of a column is not an embedding pipeline.

[Store API](../../../overlay-backend/src/persistence/sqlite_store.rs#L445-L576): inserting a candidate creates pending row, editing/rejecting modifies that candidate, and approve reads a pending row then marks approved and inserts item inside one transaction. The current approve SELECT/INSERT transfers only profile/kind/text/source_session_id, **not** candidate source_text/entity/norm_status; those V2 fields stay on the approved candidate row and default/null on the new item. Raw candidates are not injected as approved context. [Direct item insertion](../../../overlay-backend/src/persistence/sqlite_store.rs#L579-L600) is used for explicit user note/capture consent, not an automatic STT import.

[Active items query](../../../overlay-backend/src/persistence/sqlite_store.rs#L602-L637) filters `archived_at_ms IS NULL` and profile. Current UI/retrieval uses hardcoded `default`, not general multi-profile separation. [Edit/restore/archive/delete](../../../overlay-backend/src/persistence/sqlite_store.rs#L639-L687) changes approved rows only by explicit caller: restoring normalized provenance is user-triggered; archive excludes retrieval; hard delete removes row.

These user-owned tables are not reconstructed by journal index. [Session replacement](../../../overlay-backend/src/persistence/sqlite_store.rs#L84-L151) preserves curated tables; deleting the catalog loses them. No FK cascade to memory session IDs is declared in current schema.

## UI review and manual capture

[Memory tab wiring](../../../slint-experiment/src/bin/overlay_host/settings_memory.rs#L26-L150) caps displayed rows, runs SQLite mutations/list refresh on worker threads and uses refresh generation before applying Slint row models. [Manual add/edit](../../../slint-experiment/src/bin/overlay_host/settings_memory.rs#L152-L258) persists a user-authored note verbatim (`norm_status=none`) and clears submitted text only after success; newer typing is not overwritten. Display cap is not full-table scan or prompt-memory cap.

[Tile/block capture](../../../slint-experiment/src/bin/overlay_host/tile_copy.rs#L312-L356) trims outer whitespace only and inserts approved note. [Selection save](../../../slint-experiment/src/bin/overlay_host/tile_copy.rs#L447-L561) requires user save action, writes chosen block/selection and reports save success before clearing selection. It does not automatically normalize approved text. [Transcript selection surface](../../../slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs#L159-L229) invokes the same explicit capture semantics.

## Candidate mining

[extract_heuristic](../../../overlay-backend/src/memory/candidates.rs#L21-L110) creates substantial Q/A suggestions (answer at least 80 chars, top five longest) and repeated-topic suggestions from keywords in at least two questions. It has no LLM/network call itself. [Extract callback](../../../slint-experiment/src/bin/overlay_host/settings_memory.rs#L125-L150) runs this on demand behind busy flag; [run_extract](../../../slint-experiment/src/bin/overlay_host/settings_memory.rs#L326-L353) scans most recent sessions and deduplicates exact candidate text across all existing statuses.

An old module comment suggesting automatic extraction on every finished session is not enough to claim that scheduling exists. Source search in this pass found the Settings Extract production caller; no automatic stop-session insertion is asserted. Exact-string dedupe is not semantic merge or entity lifecycle.

## Live ask context

[context_for_meeting](../../../overlay-backend/src/memory/context_builder.rs#L165-L190) loads active approved default-profile rows, ranks by symmetric rooted query matching and merges a bounded formatted block with base context. All approved rows may be loaded before ranking; output limits are not input memory bounds.

[Ranking/fallback](../../../overlay-backend/src/memory/context_builder.rs#L103-L161): >=4-char terms score item text/entity, no-match or short query falls back to newest approval. This can include unrelated approved facts intentionally. [Formatter](../../../overlay-backend/src/memory/context_builder.rs#L20-L100) uses at most 8 items, block target 1200 characters, per-item 240 chars, substring instruction denylist and passive background framing. Character truncation may cut qualifications; char count is not tokenizer/model context guarantee, and base meeting context is separate.

Actual call sites include [main ask](../../../slint-experiment/src/bin/overlay_host/tile_ask.rs#L563-L592), [auto-tile](../../../slint-experiment/src/slint_session.rs#L1036-L1044), [follow-up](../../../slint-experiment/src/bin/overlay_host/tile_followup.rs#L542-L554), [PTT](../../../slint-experiment/src/bin/overlay_host/tile_ptt.rs#L379-L392) and [vision solve context](../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L778-L797). This is application prompt construction, not privileged execution of stored text.

## Summary reference and normalization boundaries

[Summary reference](../../../overlay-backend/src/memory/summary_ref.rs#L34-L39) caps five facts/800 chars/240 each and [filters by named terms](../../../overlay-backend/src/memory/summary_ref.rs#L128-L190) appearing in transcript. Its formatter does not call live ask's instruction denylist. Approved-only/passive framing reduces but does not prove absence of prompt injection. [Summary caller](../../../slint-experiment/src/bin/overlay_host_windows.rs#L2147-L2156) assembles the reference from actual meeting transcript.

[normalize_fact](../../../overlay-backend/src/memory/normalize.rs#L557-L633) is an implemented async helper selecting numbered spans and validating no-think LLM rewrite; permanent request error returns None, transient error returns Err for caller policy. [validate_rewrite](../../../overlay-backend/src/memory/normalize.rs#L493-L513) requires ordered rooted content tokens, exact digit sequence and listed negation count. That is a heuristic semantic validator, not proof of faithful meaning.

Source search in this reviewed host found **no reachable production call to normalize_fact**; explicit approved notes use verbatim path. Normalizer code/tests and legacy norm_status fields therefore do not establish automatic write-time normalization in the current capture UI. Likewise vector embeddings/RRF, entity merge/lifecycle and multi-profile behavior remain proposed/unestablished, not discovered from the mere schema placeholder.

## Failure and verification

[List read](../../../slint-experiment/src/bin/overlay_host/settings_memory.rs#L262-L324) degrades failed open/query to empty lists; some UI mutation failures are logged and refresh afterward. Empty list is not proof there is no saved memory. Explicit success/failure status and generation refresh still require live QA.

[Source-declared memory tests](../../../overlay-backend/src/memory/normalize.rs#L703-L956), [context tests](../../../overlay-backend/src/memory/context_builder.rs#L192-L344), [summary tests](../../../overlay-backend/src/memory/summary_ref.rs#L192-L284) and [store tests](../../../overlay-backend/src/persistence/sqlite_store.rs#L1004-L1343) were not natively run here. Exact-SHA acceptance should test approve/reject/edit/restore/delete, no auto-injection of candidate text, failed save with newer typing, active/profile filtering, large-store query time and adversarial approved text. See [Grok memory candidates](../reconciliation/candidates.json) for unresolved meaning/injection/relevance conditions.
