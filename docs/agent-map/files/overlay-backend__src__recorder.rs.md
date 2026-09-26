---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4a8e3266d4dd"
source_path: "overlay-backend/src/recorder.rs"
batch_id: "B07"
total_lines: 1353
symbols_count: 48
review_state: validated
---

# File Map: `overlay-backend/src/recorder.rs`

- **Batch:** B07
- **Physical Lines:** 1353
- **Coverage:** 1353/1353 lines (100%)

## Types & Structures (5)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `RecMsg` | L91 | private |
| struct | `TtsMaskSpan` | L61 | pub |
| struct | `StoredTtsMaskSpan` | L68 | private |
| struct | `SessionRecorder` | L100 | pub |
| struct | `ChannelWriter` | L255 | private |

## Symbols & Routines (48)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `start` | L119 | `fn start(session_id: &str, keep_sessions: usize, keep_days: u32, max_total_mb: u32,) -> Result<Self>` |
| function | `start_in` | L166 | `fn start_in(dir: PathBuf) -> Result<Self>` |
| function | `feed` | L189 | `fn feed(&self, chunk: &AudioChunk) -> ()` |
| function | `feed_with_tts_mask` | L197 | `fn feed_with_tts_mask(&self, chunk: &AudioChunk, tts_suppressed: bool) -> ()` |
| function | `dropped_chunks` | L216 | `fn dropped_chunks(&self) -> u64` |
| function | `dir` | L222 | `fn dir(&self) -> &Path` |
| function | `drop` | L228 | `fn drop(&mut self) -> ()` |
| function | `new` | L271 | `fn new() -> Self` |
| function | `write_chunk` | L285 | `fn write_chunk(&mut self, dir: &Path, name: &str, spec: hound::WavSpec, pcm: &[i16], timestamp_ms: u64,) -> Option<(u64, u64)>` |
| function | `finalize` | L346 | `fn finalize(self, name: &str) -> ()` |
| function | `plan_pad` | L368 | `fn plan_pad(written: u64, skew: u64, timestamp_ms: u64, chunk_len: u64, pad_budget: u64,) -> (u64, u64)` |
| function | `writer_loop` | L385 | `fn writer_loop(rx: &Receiver<RecMsg>, dir: &Path) -> ()` |
| function | `load_tts_mask_in` | L449 | `fn load_tts_mask_in(session_dir: &Path) -> Result<Vec<TtsMaskSpan>>` |
| function | `recordings_dir` | L486 | `fn recordings_dir() -> Result<PathBuf>` |
| function | `prune_old_recordings_in` | L501 | `fn prune_old_recordings_in(root: &Path, keep: usize, min_age: std::time::Duration,) -> Result<usize>` |
| function | `prune_recordings_older_than_in` | L553 | `fn prune_recordings_older_than_in(root: &Path, max_age_days: u32, min_age: std::time::Duration,) -> Result<usize>` |
| function | `prune_recordings_older_than_at` | L564 | `fn prune_recordings_older_than_at(root: &Path, max_age_days: u32, min_age: std::time::Duration, now: std::time::SystemTime,) -> Result<usize>` |
| function | `prune_recordings_over_total_in` | L615 | `fn prune_recordings_over_total_in(root: &Path, max_total_mb: u32, min_age: std::time::Duration,) -> Result<usize>` |
| function | `dir_size_bytes` | L668 | `fn dir_size_bytes(dir: &Path) -> u64` |
| function | `repair_unfinalized_in` | L691 | `fn repair_unfinalized_in(root: &Path, min_age: std::time::Duration) -> Result<usize>` |
| function | `repair_wav_header` | L742 | `fn repair_wav_header(path: &Path) -> Result<bool>` |
| function | `chunk` | L794 | `fn chunk(source: AudioSource, samples: &[i16]) -> AudioChunk` |
| function | `chunk_ts` | L802 | `fn chunk_ts(source: AudioSource, samples: &[i16], timestamp_ms: u64) -> AudioChunk` |
| function | `pp` | L814 | `fn pp(written: u64, skew: u64, timestamp_ms: u64, chunk_len: u64) -> (u64, u64)` |
| function | `plan_pad_no_gap_when_wav_tracks_wall_clock` | L819 | `fn plan_pad_no_gap_when_wav_tracks_wall_clock() -> ()` |
| function | `plan_pad_fills_a_silence_gap` | L828 | `fn plan_pad_fills_a_silence_gap() -> ()` |
| function | `plan_pad_zero_timestamp_appends` | L836 | `fn plan_pad_zero_timestamp_appends() -> ()` |
| function | `plan_pad_forward_only_on_backwards_timestamp` | L844 | `fn plan_pad_forward_only_on_backwards_timestamp() -> ()` |
| function | `plan_pad_caps_gap_and_absorbs_excess_into_skew` | L851 | `fn plan_pad_caps_gap_and_absorbs_excess_into_skew() -> ()` |
| function | `plan_pad_session_pad_budget_caps_total` | L871 | `fn plan_pad_session_pad_budget_caps_total() -> ()` |
| function | `feed_pads_silence_so_the_wav_is_a_wall_clock_timeline` | L880 | `fn feed_pads_silence_so_the_wav_is_a_wall_clock_timeline() -> ()` |
| function | `feed_pads_each_channel_on_its_own_wall_clock` | L906 | `fn feed_pads_each_channel_on_its_own_wall_clock() -> ()` |
| function | `size_cap_prunes_oldest_until_under_budget` | L933 | `fn size_cap_prunes_oldest_until_under_budget() -> ()` |
| function | `records_two_channels_to_separate_finalised_wavs` | L964 | `fn records_two_channels_to_separate_finalised_wavs() -> ()` |
| function | `failed_channel_never_retries_create_so_it_cannot_truncate` | L987 | `fn failed_channel_never_retries_create_so_it_cannot_truncate() -> ()` |
| function | `channel_with_no_audio_creates_no_file` | L1011 | `fn channel_with_no_audio_creates_no_file() -> ()` |
| function | `repair_fixes_a_crash_truncated_header` | L1026 | `fn repair_fixes_a_crash_truncated_header() -> ()` |
| function | `repair_skips_recent_files_under_grace` | L1083 | `fn repair_skips_recent_files_under_grace() -> ()` |
| function | `repair_leaves_foreign_non_canonical_wav_untouched` | L1121 | `fn repair_leaves_foreign_non_canonical_wav_untouched() -> ()` |
| function | `prune_keeps_newest_and_removes_older` | L1151 | `fn prune_keeps_newest_and_removes_older() -> ()` |
| function | `prune_skips_recently_written_dirs_under_grace` | L1181 | `fn prune_skips_recently_written_dirs_under_grace() -> ()` |
| function | `repair_ignores_missing_root_and_non_wav` | L1201 | `fn repair_ignores_missing_root_and_non_wav() -> ()` |
| function | `age_prune_zero_days_is_noop_and_fresh_dirs_survive` | L1217 | `fn age_prune_zero_days_is_noop_and_fresh_dirs_survive() -> ()` |
| function | `age_prune_removes_dirs_older_than_limit` | L1240 | `fn age_prune_removes_dirs_older_than_limit() -> ()` |
| function | `age_prune_keeps_dir_at_exact_age_boundary` | L1260 | `fn age_prune_keeps_dir_at_exact_age_boundary() -> ()` |
| function | `age_prune_grace_window_beats_age_limit` | L1277 | `fn age_prune_grace_window_beats_age_limit() -> ()` |
| function | `tts_mask_records_written_offsets_without_modifying_raw_wav` | L1298 | `fn tts_mask_records_written_offsets_without_modifying_raw_wav() -> ()` |
| function | `tts_mask_uses_the_post_padding_sample_offset` | L1326 | `fn tts_mask_uses_the_post_padding_sample_offset() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L169
