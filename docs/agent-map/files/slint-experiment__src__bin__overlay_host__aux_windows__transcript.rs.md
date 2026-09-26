---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ef896c2b73d4"
source_path: "slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs"
batch_id: "B03"
total_lines: 1534
symbols_count: 25
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs`

- **Batch:** B03
- **Physical Lines:** 1534
- **Coverage:** 1534/1534 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `DiarFailure` | L500 | private |
| struct | `DiarJobHandles` | L474 | private |
| struct | `DiarInstallSlot` | L490 | private |

## Symbols & Routines (25)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `fmt_offset` | L14 | `fn fmt_offset(offset_ms: i64) -> String` |
| function | `copy_to_clipboard_and_flash` | L26 | `fn copy_to_clipboard_and_flash(w: &TranscriptWindow, text: &str) -> ()` |
| function | `clear_transcript_marks` | L54 | `fn clear_transcript_marks(m: &VecModel<TranscriptLine>) -> ()` |
| function | `wire_transcript_actions` | L65 | `fn wire_transcript_actions(win: &TranscriptWindow, model: &Rc<VecModel<TranscriptLine>>, utts: &[Utterance], session_start: Option<i64>,) -> ()` |
| function | `active_line_for_ms` | L322 | `fn active_line_for_ms(model: &Rc<VecModel<TranscriptLine>>, pos_ms: i64) -> i32` |
| function | `wire_transcript_player` | L342 | `fn wire_transcript_player(win: &TranscriptWindow, session_id: &str, model: &Rc<VecModel<TranscriptLine>>,) -> ()` |
| function | `speaker_palette` | L513 | `fn speaker_palette(id: i32) -> slint::Color` |
| function | `neutral_speaker_color` | L529 | `fn neutral_speaker_color() -> slint::Color` |
| function | `speaker_label` | L534 | `fn speaker_label(id: i32, names: &std::collections::BTreeMap<i32, String>) -> String` |
| function | `apply_role_labels` | L544 | `fn apply_role_labels(model: &Rc<VecModel<TranscriptLine>>, utts: &[Utterance]) -> ()` |
| function | `apply_voice_labels` | L562 | `fn apply_voice_labels(model: &Rc<VecModel<TranscriptLine>>, utts: &[Utterance], diar: &Diarization,) -> ()` |
| function | `speaker_rows` | L596 | `fn speaker_rows(diar: &Diarization, utts: &[Utterance]) -> Vec<SpeakerRow>` |
| function | `set_speaker_list` | L610 | `fn set_speaker_list(win: &TranscriptWindow, diar: &Diarization, utts: &[Utterance]) -> ()` |
| function | `start_diar_poll` | L624 | `fn start_diar_poll(weak: slint::Weak<TranscriptWindow>, store: StoreSlot, diar: Rc<RefCell<Option<Diarization>>>, model: Rc<VecModel<TranscriptLine>>, utts: Rc<Vec<Utterance>>, handles: DiarJobHandles, paint_session_id: String,) -> ()` |
| function | `start_diar_install_poll` | L722 | `fn start_diar_install_poll(weak: slint::Weak<TranscriptWindow>, slot: DiarInstallSlot) -> ()` |
| function | `transcript_search_hits` | L778 | `fn transcript_search_hits(texts: &[&str], query: &str) -> Vec<usize>` |
| function | `next_hit_index` | L793 | `fn next_hit_index(cur: i32, dir: i32, n: i32) -> i32` |
| function | `wire_transcript_search` | L809 | `fn wire_transcript_search(win: &TranscriptWindow, model: &Rc<VecModel<TranscriptLine>>) -> ()` |
| function | `wire_transcript_diarization` | L880 | `fn wire_transcript_diarization(win: &TranscriptWindow, store: &StoreSlot, rt_handle: &tokio::runtime::Handle, session_id: &str, session_finished: bool, utts_display: &[Utterance], model: &Rc<VecModel<TranscriptLine>>,) -> ()` |
| function | `open_transcript` | L1276 | `fn open_transcript(slot: &Rc<RefCell<Option<TranscriptWindow>>>, session: Option<&Session>, utts: &[Utterance], store: &StoreSlot, rt_handle: &tokio::runtime::Handle,) -> ()` |
| function | `diar_latch_is_a_single_job_gate` | L1462 | `fn diar_latch_is_a_single_job_gate() -> ()` |
| function | `diar_failure_messages_are_distinct_and_non_empty` | L1491 | `fn diar_failure_messages_are_distinct_and_non_empty() -> ()` |
| function | `transcript_search_hits_case_insensitive_incl_cyrillic` | L1498 | `fn transcript_search_hits_case_insensitive_incl_cyrillic() -> ()` |
| function | `next_hit_index_fresh_and_wrap` | L1512 | `fn next_hit_index_fresh_and_wrap() -> ()` |
| function | `fmt_offset_mm_ss_and_h_mm_ss` | L1528 | `fn fmt_offset_mm_ss_and_h_mm_ss() -> ()` |
