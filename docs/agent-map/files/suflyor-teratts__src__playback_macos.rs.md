---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_d85ad53c03fd"
source_path: "suflyor-teratts/src/playback_macos.rs"
batch_id: "B12"
total_lines: 513
symbols_count: 27
review_state: validated
---

# File Map: `suflyor-teratts/src/playback_macos.rs`

- **Batch:** B12
- **Physical Lines:** 513
- **Coverage:** 513/513 lines (100%)

## Types & Structures (6)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `EmptyQueueAction` | L16 | private |
| enum | `PlaybackControl` | L39 | private |
| struct | `Playback` | L30 | pub |
| struct | `ExitNotifier` | L44 | private |
| struct | `BufferedTimeline` | L70 | private |
| struct | `ContinuousResampler` | L279 | private |

## Symbols & Routines (27)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `empty_queue_action` | L22 | `fn empty_queue_action(end_of_stream: bool, pending_samples: usize) -> EmptyQueueAction` |
| function | `drop` | L47 | `fn drop(&mut self) -> ()` |
| function | `should_start_stream` | L59 | `fn should_start_stream(pending_samples: usize, sample_rate: u32, end_of_stream: bool, source_drained: bool,) -> bool` |
| function | `new` | L77 | `fn new() -> Self` |
| function | `available` | L84 | `fn available(&self) -> usize` |
| function | `append` | L89 | `fn append(&mut self, samples: Vec<f32>) -> ()` |
| function | `take` | L92 | `fn take(&mut self, count: usize) -> Vec<f32>` |
| function | `seek_seconds` | L105 | `fn seek_seconds(&mut self, seconds: i32, sample_rate: u32) -> ()` |
| function | `rewind_to` | L115 | `fn rewind_to(&mut self, checkpoint: u64) -> ()` |
| function | `prune_played_history` | L119 | `fn prune_played_history(&mut self, sample_rate: u32) -> ()` |
| function | `feed` | L158 | `fn feed(&self, samples: Vec<f32>) -> ()` |
| function | `end_of_stream` | L165 | `fn end_of_stream(&self) -> ()` |
| function | `pause` | L169 | `fn pause(&self) -> ()` |
| function | `resume` | L173 | `fn resume(&self) -> ()` |
| function | `seek_seconds` | L177 | `fn seek_seconds(&self, seconds: i32) -> ()` |
| function | `set_speed` | L181 | `fn set_speed(&self, speed: f32) -> ()` |
| function | `stop` | L185 | `fn stop(mut self) -> ()` |
| function | `drop` | L194 | `fn drop(&mut self) -> ()` |
| function | `drain_feed` | L202 | `fn drain_feed(feed_rx: &Receiver<Vec<f32>>, timeline: &mut BufferedTimeline) -> ()` |
| function | `drain_controls` | L209 | `fn drain_controls(control_rx: &Receiver<PlaybackControl>, sample_rate: u32, timeline: &mut BufferedTimeline, output: &mut VecDeque<f32>, output_checkpoint: &mut Option<u64>, speed: &mut f32, stretcher: &mut Option<suflyor_wsola::StreamingWsola>, stretch_finished: &mut bool,) -> ()` |
| function | `fill_output` | L238 | `fn fill_output(sample_rate: u32, speed: f32, eos: bool, timeline: &mut BufferedTimeline, output: &mut VecDeque<f32>, output_checkpoint: &mut Option<u64>, stretcher: &mut Option<suflyor_wsola::StreamingWsola>, stretch_finished: &mut bool,) -> ()` |
| function | `new` | L287 | `fn new(src_rate: u32, dst_rate: u32) -> Self` |
| function | `process` | L296 | `fn process(&mut self, input: &[f32], output: &mut VecDeque<f32>) -> ()` |
| function | `render_loop` | L335 | `fn render_loop(sample_rate: u32, feed_rx: Receiver<Vec<f32>>, control_rx: Receiver<PlaybackControl>, eos: Arc<AtomicBool>, stop: Arc<AtomicBool>, paused: Arc<AtomicBool>,) -> Result<()>` |
| function | `end_of_stream_drains_without_refilling_silence` | L488 | `fn end_of_stream_drains_without_refilling_silence() -> ()` |
| function | `exit_notifier_runs_when_its_scope_ends` | L495 | `fn exit_notifier_runs_when_its_scope_ends() -> ()` |
| function | `stream_waits_for_prebuffer_but_short_eos_still_starts` | L507 | `fn stream_waits_for_prebuffer_but_short_eos_still_starts() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L131
- Instantiates IPC channel at L132
