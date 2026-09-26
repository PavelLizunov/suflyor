---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_a775bc5d75ec"
source_path: "slint-experiment/src/bin/overlay_host/tile_copy.rs"
batch_id: "B03"
total_lines: 1311
symbols_count: 58
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/tile_copy.rs`

- **Batch:** B03
- **Physical Lines:** 1311
- **Coverage:** 1311/1311 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (58)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `message_text` | L50 | `fn message_text(content: &ai::MessageContent) -> String` |
| function | `format_transcript_for_copy` | L75 | `fn format_transcript_for_copy(utts: &[overlay_backend::persistence::Utterance], session_start_ms: Option<i64>, selected: Option<&std::collections::HashSet<usize>>, with_timecodes: bool,) -> String` |
| function | `user_question_for_copy` | L127 | `fn user_question_for_copy(raw: &str) -> String` |
| function | `strip_followup_directives` | L154 | `fn strip_followup_directives(messages: &mut [ai::ChatMessage]) -> ()` |
| function | `convo_copy_text` | L179 | `fn convo_copy_text(bridge: &OverlayBarBridge, convo_id: i32) -> String` |
| function | `format_convo_copy` | L194 | `fn format_convo_copy(messages: &[ai::ChatMessage], rendered: &str) -> String` |
| function | `wire_copy` | L249 | `fn wire_copy(tile: &TileWindow, convo_id: i32, bridge: &Arc<OverlayBarBridge>) -> ()` |
| function | `wire_code_copy` | L283 | `fn wire_code_copy(tile: &TileWindow) -> ()` |
| function | `insert_approved_note` | L315 | `fn insert_approved_note(text: &str) -> ()` |
| function | `approved_note_text` | L321 | `fn approved_note_text(text: &str) -> Option<&str>` |
| function | `store_note` | L327 | `fn store_note(text: &str, source_text: Option<&str>, norm_status: &str) -> Option<i64>` |
| function | `wire_block_capture` | L364 | `fn wire_block_capture(tile: &TileWindow) -> ()` |
| function | `char_boundary` | L575 | `fn char_boundary(s: &str, i: usize) -> usize` |
| function | `clear_all_marks` | L584 | `fn clear_all_marks(vm: &VecModel<MarkdownBlock>) -> ()` |
| function | `join_marked_text` | L596 | `fn join_marked_text(vm: &VecModel<MarkdownBlock>) -> String` |
| function | `build_select_text` | L618 | `fn build_select_text(vm: &VecModel<MarkdownBlock>) -> String` |
| function | `convo_speak_text` | L654 | `fn convo_speak_text(bridge: &OverlayBarBridge, convo_id: i32) -> String` |
| function | `speak_answer_text` | L666 | `fn speak_answer_text(messages: &[ai::ChatMessage], rendered: &str) -> String` |
| function | `sync_speaking_tiles` | L699 | `fn sync_speaking_tiles(active: Option<i32>) -> ()` |
| function | `clear_speaking_tile` | L722 | `fn clear_speaking_tile() -> ()` |
| function | `start_speaking_state_timer` | L729 | `fn start_speaking_state_timer() -> ()` |
| function | `mark_speaking` | L745 | `fn mark_speaking(convo_id: i32) -> ()` |
| function | `set_speak_error` | L751 | `fn set_speak_error(convo_id: i32, failed: bool) -> ()` |
| function | `speak_explicit` | L767 | `fn speak_explicit(text: &str, convo_id: i32) -> bool` |
| function | `stop_if_speaking` | L780 | `fn stop_if_speaking(convo_id: i32) -> ()` |
| function | `current_speaking_convo` | L788 | `fn current_speaking_convo() -> i32` |
| function | `speak_speed_label` | L805 | `fn speak_speed_label(percent: u16) -> String` |
| function | `next_speak_speed_percent` | L810 | `fn next_speak_speed_percent(current: u16) -> u16` |
| function | `set_speak_speed_percent` | L819 | `fn set_speak_speed_percent(percent: u16) -> ()` |
| function | `toggle_pause` | L826 | `fn toggle_pause() -> bool` |
| function | `reset_pause` | L837 | `fn reset_pause() -> ()` |
| function | `wire_speak` | L845 | `fn wire_speak(tile: &TileWindow, convo_id: i32, bridge: &Arc<OverlayBarBridge>) -> ()` |
| function | `tile_player_speed_cycle_returns_to_normal` | L922 | `fn tile_player_speed_cycle_returns_to_normal() -> ()` |
| function | `char_boundary_clamps_into_multibyte` | L932 | `fn char_boundary_clamps_into_multibyte() -> ()` |
| function | `select_text_join_prefixes_bullets_and_skips_hr` | L947 | `fn select_text_join_prefixes_bullets_and_skips_hr() -> ()` |
| function | `select_text_collapses_newlines_and_char_caps` | L970 | `fn select_text_collapses_newlines_and_char_caps() -> ()` |
| function | `join_marked_text_combines_marked_only_in_order` | L996 | `fn join_marked_text_combines_marked_only_in_order() -> ()` |
| function | `approved_note_preserves_structured_schedule_verbatim` | L1018 | `fn approved_note_preserves_structured_schedule_verbatim() -> ()` |
| function | `transcript_copy_format` | L1032 | `fn transcript_copy_format() -> ()` |
| function | `msg` | L1082 | `fn msg(role: &str, text: &str) -> ai::ChatMessage` |
| function | `parts_msg` | L1088 | `fn parts_msg(role: &str, texts: &[&str]) -> ai::ChatMessage` |
| function | `message_text_text_and_parts` | L1103 | `fn message_text_text_and_parts() -> ()` |
| function | `copy_question_strips_transcript_wrapper` | L1115 | `fn copy_question_strips_transcript_wrapper() -> ()` |
| function | `conversations_evict_keys_drops_oldest_half_keeps_newest` | L1122 | `fn conversations_evict_keys_drops_oldest_half_keeps_newest() -> ()` |
| function | `copy_question_drops_transcript_only_ask` | L1146 | `fn copy_question_drops_transcript_only_ask() -> ()` |
| function | `copy_question_strips_followup_directive` | L1152 | `fn copy_question_strips_followup_directive() -> ()` |
| function | `copy_question_drops_canned_vision_prompt` | L1158 | `fn copy_question_drops_canned_vision_prompt() -> ()` |
| function | `copy_question_drops_translate_vision_prompt` | L1163 | `fn copy_question_drops_translate_vision_prompt() -> ()` |
| function | `copy_question_passes_plain_text_trimmed` | L1172 | `fn copy_question_passes_plain_text_trimmed() -> ()` |
| function | `single_turn_copies_only_the_answer` | L1177 | `fn single_turn_copies_only_the_answer() -> ()` |
| function | `multi_turn_copies_labelled_thread_without_transcript` | L1190 | `fn multi_turn_copies_labelled_thread_without_transcript() -> ()` |
| function | `multi_turn_vision_skips_canned_prompt` | L1211 | `fn multi_turn_vision_skips_canned_prompt() -> ()` |
| function | `empty_conversation_falls_back_to_rendered` | L1226 | `fn empty_conversation_falls_back_to_rendered() -> ()` |
| function | `speak_reads_latest_answer_only_not_prompts_or_old_turns` | L1231 | `fn speak_reads_latest_answer_only_not_prompts_or_old_turns() -> ()` |
| function | `speak_falls_back_to_rendered_before_any_answer` | L1248 | `fn speak_falls_back_to_rendered_before_any_answer() -> ()` |
| function | `speak_read_aloud_tile_uses_raw_selected_text_not_visual_fence` | L1258 | `fn speak_read_aloud_tile_uses_raw_selected_text_not_visual_fence() -> ()` |
| function | `strip_directives_cleans_user_turns_only` | L1267 | `fn strip_directives_cleans_user_turns_only() -> ()` |
| function | `strip_all_but_last_preserves_reasked_turn` | L1290 | `fn strip_all_but_last_preserves_reasked_turn() -> ()` |
