---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_8c18d742ed58"
source_path: "suflyor-teratts/src/playback.rs"
batch_id: "B12"
total_lines: 470
symbols_count: 25
review_state: validated
---

# File Map: `suflyor-teratts/src/playback.rs`

- **Batch:** B12
- **Physical Lines:** 470
- **Coverage:** 470/470 lines (100%)

## Types & Structures (4)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `EmptyQueueAction` | L28 | private |
| enum | `PlaybackControl` | L52 | private |
| struct | `Playback` | L43 | pub |
| struct | `BufferedTimeline` | L62 | private |

## Symbols & Routines (25)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `samples_to_bytes` | L19 | `fn samples_to_bytes(samples: &[f32]) -> Vec<u8>` |
| function | `empty_queue_action` | L34 | `fn empty_queue_action(end_of_stream: bool, padding: usize) -> EmptyQueueAction` |
| function | `new` | L69 | `fn new() -> Self` |
| function | `end` | L77 | `fn end(&self) -> u64` |
| function | `append` | L81 | `fn append(&mut self, samples: Vec<f32>) -> ()` |
| function | `available` | L85 | `fn available(&self) -> usize` |
| function | `take` | L89 | `fn take(&mut self, count: usize) -> Vec<f32>` |
| function | `seek_seconds` | L105 | `fn seek_seconds(&mut self, seconds: i32, sample_rate: u32) -> ()` |
| function | `rewind_to` | L115 | `fn rewind_to(&mut self, checkpoint: u64) -> ()` |
| function | `prune_played_history` | L119 | `fn prune_played_history(&mut self, sample_rate: u32) -> ()` |
| function | `feed` | L163 | `fn feed(&self, samples: Vec<f32>) -> ()` |
| function | `end_of_stream` | L170 | `fn end_of_stream(&self) -> ()` |
| function | `pause` | L174 | `fn pause(&self) -> ()` |
| function | `resume` | L178 | `fn resume(&self) -> ()` |
| function | `seek_seconds` | L182 | `fn seek_seconds(&self, seconds: i32) -> ()` |
| function | `set_speed` | L186 | `fn set_speed(&self, speed: f32) -> ()` |
| function | `stop` | L190 | `fn stop(mut self) -> ()` |
| function | `drop` | L199 | `fn drop(&mut self) -> ()` |
| function | `render_loop` | L207 | `fn render_loop(sample_rate: u32, feed_rx: Receiver<Vec<f32>>, control_rx: Receiver<PlaybackControl>, eos: Arc<AtomicBool>, stop: Arc<AtomicBool>, paused: Arc<AtomicBool>,) -> Result<()>` |
| function | `drain_feed` | L352 | `fn drain_feed(feed_rx: &Receiver<Vec<f32>>, timeline: &mut BufferedTimeline) -> ()` |
| function | `drain_controls` | L359 | `fn drain_controls(control_rx: &Receiver<PlaybackControl>, sample_rate: u32, timeline: &mut BufferedTimeline, output: &mut VecDeque<f32>, output_checkpoint: &mut Option<u64>, speed: &mut f32, stretcher: &mut Option<suflyor_wsola::StreamingWsola>, stretch_finished: &mut bool,) -> ()` |
| function | `fill_output` | L388 | `fn fill_output(sample_rate: u32, speed: f32, eos: bool, timeline: &mut BufferedTimeline, output: &mut VecDeque<f32>, output_checkpoint: &mut Option<u64>, stretcher: &mut Option<suflyor_wsola::StreamingWsola>, stretch_finished: &mut bool,) -> ()` |
| function | `end_of_stream_drains_without_refilling_silence` | L435 | `fn end_of_stream_drains_without_refilling_silence() -> ()` |
| function | `seek_clamps_to_retained_history_and_buffered_horizon` | L442 | `fn seek_clamps_to_retained_history_and_buffered_horizon() -> ()` |
| function | `played_history_is_bounded_but_future_pcm_is_kept` | L458 | `fn played_history_is_bounded_but_future_pcm_is_kept() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L133
- Instantiates IPC channel at L134
