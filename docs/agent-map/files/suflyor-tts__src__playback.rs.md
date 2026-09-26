---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_831d09d04121"
source_path: "suflyor-tts/src/playback.rs"
batch_id: "B11"
total_lines: 472
symbols_count: 27
review_state: validated
---

# File Map: `suflyor-tts/src/playback.rs`

- **Batch:** B11
- **Physical Lines:** 472
- **Coverage:** 472/472 lines (100%)

## Types & Structures (5)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `EmptyQueueAction` | L26 | private |
| enum | `PlaybackControl` | L52 | private |
| struct | `Playback` | L43 | pub |
| struct | `ExitNotifier` | L57 | private |
| struct | `BufferedTimeline` | L70 | private |

## Symbols & Routines (27)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `samples_to_bytes` | L17 | `fn samples_to_bytes(samples: &[f32]) -> Vec<u8>` |
| function | `empty_queue_action` | L34 | `fn empty_queue_action(end_of_stream: bool, padding: usize) -> EmptyQueueAction` |
| function | `drop` | L60 | `fn drop(&mut self) -> ()` |
| function | `new` | L77 | `fn new() -> Self` |
| function | `end` | L84 | `fn end(&self) -> u64` |
| function | `append` | L87 | `fn append(&mut self, samples: Vec<f32>) -> ()` |
| function | `available` | L90 | `fn available(&self) -> usize` |
| function | `take` | L93 | `fn take(&mut self, count: usize) -> Vec<f32>` |
| function | `seek_seconds` | L106 | `fn seek_seconds(&mut self, seconds: i32, sample_rate: u32) -> ()` |
| function | `rewind_to` | L115 | `fn rewind_to(&mut self, checkpoint: u64) -> ()` |
| function | `prune_played_history` | L118 | `fn prune_played_history(&mut self, sample_rate: u32) -> ()` |
| function | `feed` | L160 | `fn feed(&self, samples: Vec<f32>) -> ()` |
| function | `end_of_stream` | L167 | `fn end_of_stream(&self) -> ()` |
| function | `pause` | L171 | `fn pause(&self) -> ()` |
| function | `resume` | L175 | `fn resume(&self) -> ()` |
| function | `seek_seconds` | L179 | `fn seek_seconds(&self, seconds: i32) -> ()` |
| function | `set_speed` | L183 | `fn set_speed(&self, speed: f32) -> ()` |
| function | `stop` | L187 | `fn stop(mut self) -> ()` |
| function | `drop` | L196 | `fn drop(&mut self) -> ()` |
| function | `render_loop` | L204 | `fn render_loop(sample_rate: u32, feed_rx: Receiver<Vec<f32>>, control_rx: Receiver<PlaybackControl>, eos: Arc<AtomicBool>, stop: Arc<AtomicBool>, paused: Arc<AtomicBool>,) -> Result<()>` |
| function | `drain_feed` | L350 | `fn drain_feed(feed_rx: &Receiver<Vec<f32>>, timeline: &mut BufferedTimeline) -> ()` |
| function | `drain_controls` | L357 | `fn drain_controls(control_rx: &Receiver<PlaybackControl>, sample_rate: u32, timeline: &mut BufferedTimeline, output: &mut VecDeque<f32>, output_checkpoint: &mut Option<u64>, speed: &mut f32, stretcher: &mut Option<suflyor_wsola::StreamingWsola>, stretch_finished: &mut bool,) -> ()` |
| function | `fill_output` | L386 | `fn fill_output(sample_rate: u32, speed: f32, eos: bool, timeline: &mut BufferedTimeline, output: &mut VecDeque<f32>, output_checkpoint: &mut Option<u64>, stretcher: &mut Option<suflyor_wsola::StreamingWsola>, stretch_finished: &mut bool,) -> ()` |
| function | `end_of_stream_drains_without_refilling_silence` | L432 | `fn end_of_stream_drains_without_refilling_silence() -> ()` |
| function | `exit_notifier_runs_when_its_scope_ends` | L439 | `fn exit_notifier_runs_when_its_scope_ends() -> ()` |
| function | `seek_clamps_to_retained_history_and_buffered_horizon` | L451 | `fn seek_clamps_to_retained_history_and_buffered_horizon() -> ()` |
| function | `played_history_is_bounded_without_discarding_future_audio` | L464 | `fn played_history_is_bounded_without_discarding_future_audio() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L132
- Instantiates IPC channel at L133
