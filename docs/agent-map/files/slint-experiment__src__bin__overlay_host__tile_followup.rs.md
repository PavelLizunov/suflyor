---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_9e27922fa3b3"
source_path: "slint-experiment/src/bin/overlay_host/tile_followup.rs"
batch_id: "B03"
total_lines: 988
symbols_count: 15
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/tile_followup.rs`

- **Batch:** B03
- **Physical Lines:** 988
- **Coverage:** 988/988 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (15)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `user_turn_markdown` | L28 | `fn user_turn_markdown(question: &str) -> String` |
| function | `followup_system_prompt` | L61 | `fn followup_system_prompt(response_language: &str, meeting_context: &str, prior_context: &str,) -> String` |
| function | `strip_transcript_scaffold` | L117 | `fn strip_transcript_scaffold(s: &str) -> String` |
| function | `reframe_for_send` | L137 | `fn reframe_for_send(history: &[ai::ChatMessage], response_language: &str, meeting_context: &str,) -> Vec<ai::ChatMessage>` |
| function | `wire_voice_followup` | L214 | `fn wire_voice_followup(tile: &TileWindow, convo_id: i32, route: LiveRoute, cfg: &overlay_backend::config::SharedConfig,) -> ()` |
| function | `wire_escalate` | L332 | `fn wire_escalate(tile: &TileWindow, convo_id: i32, route: &LiveRoute, bridge: &Arc<OverlayBarBridge>, events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle,) -> ()` |
| function | `fire_regenerate` | L635 | `fn fire_regenerate(convo_id: i32, tile_weak: slint::Weak<TileWindow>, bridge: &Arc<OverlayBarBridge>, events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle, route: AskRoute,) -> ()` |
| function | `user_turn_uses_a_fence_longer_than_pasted_code` | L865 | `fn user_turn_uses_a_fence_longer_than_pasted_code() -> ()` |
| function | `msg` | L871 | `fn msg(role: &str, text: &str) -> ai::ChatMessage` |
| function | `text_of` | L877 | `fn text_of(m: &ai::ChatMessage) -> String` |
| function | `reframe_collapses_text_followup_to_single_user_turn_with_context` | L888 | `fn reframe_collapses_text_followup_to_single_user_turn_with_context() -> ()` |
| function | `reframe_leaves_vision_dialog_multiturn` | L926 | `fn reframe_leaves_vision_dialog_multiturn() -> ()` |
| function | `strip_transcript_scaffold_yields_plain_question` | L947 | `fn strip_transcript_scaffold_yields_plain_question() -> ()` |
| function | `followup_system_prompt_carries_language_context_and_prior` | L963 | `fn followup_system_prompt_carries_language_context_and_prior() -> ()` |
| function | `followup_system_prompt_carries_role_style_semantics` | L974 | `fn followup_system_prompt_carries_role_style_semantics() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L287
- Spawns asynchronous thread/task at L524
- Spawns asynchronous thread/task at L740
