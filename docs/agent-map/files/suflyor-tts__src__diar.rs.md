---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_a1d180f259fa"
source_path: "suflyor-tts/src/diar.rs"
batch_id: "B11"
total_lines: 364
symbols_count: 17
review_state: validated
---

# File Map: `suflyor-tts/src/diar.rs`

- **Batch:** B11
- **Physical Lines:** 364
- **Coverage:** 364/364 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `Segment` | L26 | pub |
| struct | `Args` | L54 | private |

## Symbols & Routines (17)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `run_cli` | L35 | `fn run_cli(args: &[String]) -> i32` |
| function | `run` | L48 | `fn run(args: &[String]) -> Result<String>` |
| function | `parse_args` | L66 | `fn parse_args(args: &[String]) -> Result<Args>` |
| function | `next_val` | L106 | `fn next_val(args: &'a [String], i: &mut usize, flag: &str) -> Result<&'a str>` |
| function | `diarize` | L119 | `fn diarize(wav: &Path, seg: &Path, emb: &Path, num_speakers: i32, threshold: f32,) -> Result<(i32, Vec<Segment>)>` |
| function | `postprocess` | L191 | `fn postprocess(segments: Vec<Segment>) -> Vec<Segment>` |
| function | `to_json` | L220 | `fn to_json(num_speakers: i32, segments: &[Segment]) -> String` |
| function | `to_json_matches_the_contract` | L246 | `fn to_json_matches_the_contract() -> ()` |
| function | `parse_args_reads_positional_and_flags` | L267 | `fn parse_args_reads_positional_and_flags() -> ()` |
| function | `parse_args_requires_wav_seg_emb` | L288 | `fn parse_args_requires_wav_seg_emb() -> ()` |
| function | `parse_args_rejects_dangling_flag_and_extra_positional` | L296 | `fn parse_args_rejects_dangling_flag_and_extra_positional() -> ()` |
| function | `parse_args_rejects_flag_as_value_and_bad_numbers` | L310 | `fn parse_args_rejects_flag_as_value_and_bad_numbers() -> ()` |
| function | `seg` | L323 | `fn seg(start_ms: i64, end_ms: i64, sp: i32) -> Segment` |
| function | `postprocess_merges_same_speaker_across_short_gap` | L332 | `fn postprocess_merges_same_speaker_across_short_gap() -> ()` |
| function | `postprocess_attaches_short_fragment_to_previous` | L340 | `fn postprocess_attaches_short_fragment_to_previous() -> ()` |
| function | `postprocess_drops_isolated_short_fragment` | L352 | `fn postprocess_drops_isolated_short_fragment() -> ()` |
| function | `postprocess_keeps_sorted_nonoverlapping_segments` | L360 | `fn postprocess_keeps_sorted_nonoverlapping_segments() -> ()` |
