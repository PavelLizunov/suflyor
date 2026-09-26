---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_d4e14495f117"
source_path: "slint-experiment/src/bin/overlay_host/transcript_player.rs"
batch_id: "B03"
total_lines: 585
symbols_count: 39
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/transcript_player.rs`

- **Batch:** B03
- **Physical Lines:** 585
- **Coverage:** 585/585 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `PcmCursor` | L25 | private |
| struct | `StretchSource` | L84 | private |
| struct | `TranscriptPlayer` | L208 | pub(crate) |

## Symbols & Routines (39)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `next` | L33 | `fn next(&mut self) -> Option<f32>` |
| function | `current_span_len` | L46 | `fn current_span_len(&self) -> Option<usize>` |
| function | `channels` | L49 | `fn channels(&self) -> NonZeroU16` |
| function | `sample_rate` | L52 | `fn sample_rate(&self) -> NonZeroU32` |
| function | `total_duration` | L55 | `fn total_duration(&self) -> Option<std::time::Duration>` |
| function | `new` | L99 | `fn new(pcm: Arc<[i16]>, from: usize, sample_rate: u32, speed: f32) -> Self` |
| function | `refill` | L126 | `fn refill(&mut self) -> bool` |
| function | `next` | L182 | `fn next(&mut self) -> Option<f32>` |
| function | `current_span_len` | L193 | `fn current_span_len(&self) -> Option<usize>` |
| function | `channels` | L196 | `fn channels(&self) -> NonZeroU16` |
| function | `sample_rate` | L199 | `fn sample_rate(&self) -> NonZeroU32` |
| function | `total_duration` | L202 | `fn total_duration(&self) -> Option<std::time::Duration>` |
| function | `samples_advanced` | L234 | `fn samples_advanced(elapsed_secs: f64, sample_rate: u32, speed: f32) -> usize` |
| function | `new` | L241 | `fn new(pcm: Vec<i16>, sample_rate: u32) -> anyhow::Result<Self>` |
| function | `total` | L259 | `fn total(&self) -> usize` |
| function | `position_sample` | L264 | `fn position_sample(&self) -> usize` |
| function | `position_ms` | L276 | `fn position_ms(&self) -> i64` |
| function | `total_ms` | L281 | `fn total_ms(&self) -> i64` |
| function | `is_playing` | L286 | `fn is_playing(&self) -> bool` |
| function | `load_from` | L292 | `fn load_from(&mut self, from: usize) -> ()` |
| function | `play` | L323 | `fn play(&mut self) -> ()` |
| function | `pause` | L342 | `fn pause(&mut self) -> ()` |
| function | `toggle` | L349 | `fn toggle(&mut self) -> ()` |
| function | `seek_ms` | L359 | `fn seek_ms(&mut self, ms: i64) -> ()` |
| function | `set_speed` | L375 | `fn set_speed(&mut self, speed: f32) -> ()` |
| function | `set_volume` | L391 | `fn set_volume(&mut self, volume: f32) -> ()` |
| function | `reset` | L415 | `fn reset() -> ()` |
| function | `ensure` | L422 | `fn ensure(session_id: &str) -> bool` |
| function | `is_playing` | L444 | `fn is_playing() -> bool` |
| function | `toggle` | L453 | `fn toggle() -> ()` |
| function | `seek_and_play` | L463 | `fn seek_and_play(ms: i64) -> ()` |
| function | `seek_fraction` | L475 | `fn seek_fraction(frac: f32) -> ()` |
| function | `set_speed` | L487 | `fn set_speed(speed: f32) -> ()` |
| function | `set_volume` | L496 | `fn set_volume(volume: f32) -> ()` |
| function | `snapshot` | L506 | `fn snapshot() -> Option<(f32, i64, i64, bool)>` |
| function | `set_poll_timer` | L522 | `fn set_poll_timer(timer: slint::Timer) -> ()` |
| function | `speed_scales_playback_advance` | L534 | `fn speed_scales_playback_advance() -> ()` |
| function | `stretch_source_compresses_length_2x_and_3x_multichunk` | L544 | `fn stretch_source_compresses_length_2x_and_3x_multichunk() -> ()` |
| function | `live_rodio_device_smoke` | L571 | `fn live_rodio_device_smoke() -> ()` |
