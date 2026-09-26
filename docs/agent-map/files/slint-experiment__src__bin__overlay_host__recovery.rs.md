---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_69b63541f095"
source_path: "slint-experiment/src/bin/overlay_host/recovery.rs"
batch_id: "B01"
total_lines: 465
symbols_count: 15
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/recovery.rs`

- **Batch:** B01
- **Physical Lines:** 465
- **Coverage:** 465/465 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (15)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `build_recovery_block` | L52 | `fn build_recovery_block(recovered: &overlay_backend::journal::UnfinishedSession,) -> String` |
| function | `strip_recovery_block` | L102 | `fn strip_recovery_block(context: &str) -> String` |
| function | `compose_recovery_context` | L130 | `fn compose_recovery_context(existing_context: &str, recovered: &overlay_backend::journal::UnfinishedSession,) -> String` |
| function | `seed_recovery_context` | L157 | `fn seed_recovery_context(cfg: &overlay_backend::config::SharedConfig, recovered: &overlay_backend::journal::UnfinishedSession,) -> usize` |
| function | `open_recover_offer` | L188 | `fn open_recover_offer(slot_ref: &Rc<RefCell<Option<RecoverOfferWindow>>>, recovered: overlay_backend::journal::UnfinishedSession, cfg: &overlay_backend::config::SharedConfig, events: &Arc<dyn RuntimeEvents>, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle, state: &slint_replay::app_state::SharedState, overlay_weak: &slint::Weak<OverlayBarWindow>,) -> ()` |
| function | `recovered` | L351 | `fn recovered() -> overlay_backend::journal::UnfinishedSession` |
| function | `header_count` | L365 | `fn header_count(s: &str) -> usize` |
| function | `footer_count` | L368 | `fn footer_count(s: &str) -> usize` |
| function | `reseeding_yields_exactly_one_block` | L375 | `fn reseeding_yields_exactly_one_block() -> ()` |
| function | `reseeding_preserves_user_prose_verbatim` | L396 | `fn reseeding_preserves_user_prose_verbatim() -> ()` |
| function | `seeding_with_no_prior_block_matches_legacy` | L414 | `fn seeding_with_no_prior_block_matches_legacy() -> ()` |
| function | `strip_no_block_is_identity` | L430 | `fn strip_no_block_is_identity() -> ()` |
| function | `strip_collapses_leftover_blank_lines` | L437 | `fn strip_collapses_leftover_blank_lines() -> ()` |
| function | `strip_collapses_stacked_blocks_from_pre_guard_config` | L446 | `fn strip_collapses_stacked_blocks_from_pre_guard_config() -> ()` |
| function | `strip_keeps_text_when_footer_missing` | L459 | `fn strip_keeps_text_when_footer_missing() -> ()` |
