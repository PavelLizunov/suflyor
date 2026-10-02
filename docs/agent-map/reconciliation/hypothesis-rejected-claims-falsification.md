# Original rejected claims: bounded falsification and architectural counter-evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_rejected_claims_falsification.py) inspect frozen source definitions and architecture comments to verify the falsification of the 5 originally rejected Grok audit claims:
- `wave1_worker1_persistence-C07`
- `wave1_worker1_persistence-C11`
- `wave1_worker3_config-C10`
- `wave3_worker1_bridge-C01`
- `wave3_worker3_tile-C04`

All five remain `rejected` in `candidates.json`.

## C07 (persistence) — Catalog as an intentional additive projection

The audit claimed that `indexer` fails to delete catalog rows when `prune_old_sessions_with_size_cap` deletes old JSONL journals.
In `overlay-backend/src/persistence/mod.rs` [L12-L28](<../../../overlay-backend/src/persistence/mod.rs#L12-L28>), the documentation explicitly establishes that the SQLite catalog is designed as an additive projection:
`"This is a feature, not drift — the ~few-MB catalog is the long-term searchable history while the bulky raw journals/audio rotate out."`
Retention pruning of raw journals does not invalidate catalog entries by design.

## C11 (persistence) — Automatic FTS5 synchronization triggers

The audit claimed FTS maintenance requires manual synchronization.
In `overlay-backend/migrations/0002_fts.sql` [L19-L32](<../../../overlay-backend/migrations/0002_fts.sql#L19-L32>), full-text search entries are automatically maintained by SQLite triggers:
`AFTER INSERT ON utterances` and `AFTER INSERT ON ai_turns`, which automatically insert rows into `search_index`.

## C10 (config) — `http_error_line` static operation labels

The audit claimed `http_error_line` allowed injection via an unsanitized `op` argument.
In `overlay-backend/src/stt.rs` [L948](<../../../overlay-backend/src/stt.rs#L948>) and other call sites, the `op` argument is exclusively invoked with static string literals (e.g. `"STT"`).

## C01 (bridge) — Non-blocking event dispatch via `invoke_from_event_loop`

The audit claimed a deadlock between Slint and Tokio caused by synchronous blocking while holding `Mutex<SlintRuntime>`.
In `slint-experiment/src/bin/overlay_host/tile_controller.rs` [L819-L835](<../../../slint-experiment/src/bin/overlay_host/tile_controller.rs#L819-L835>), the bridge extracts state with an immediate lock drop (`slot.take()`) and posts the UI update non-blockingly via `slint::invoke_from_event_loop`.

## C04 (tile) — System prompt exclusion from clipboard text

The audit claimed that copying a tile copies internal system prompts.
In `slint-experiment/src/bin/overlay_host/tile_copy.rs` [L194-L225](<../../../slint-experiment/src/bin/overlay_host/tile_copy.rs#L194-L225>), `format_convo_copy` explicitly filters out system messages (`.filter(|m| m.role != "system")`) and cleans user questions via `user_question_for_copy`.

## Limits

No native platform clipboard operations were executed and no live database mutations were performed. Original statuses in `candidates.json` remain `rejected`.
