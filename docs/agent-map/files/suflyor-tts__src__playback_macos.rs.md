---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_5e78950e9966"
source_path: "suflyor-tts/src/playback_macos.rs"
batch_id: "B11"
total_lines: 474
symbols_count: 25
review_state: validated
---

# File Map: `suflyor-tts/src/playback_macos.rs`

- **Batch:** B11
- **Physical Lines:** 474
- **Coverage:** 474/474 lines (100%)

## Types & Structures (6)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `EmptyQueueAction` | L16 | private |
| enum | `PlaybackControl` | L39 | private |
| struct | `Playback` | L30 | pub |
| struct | `ExitNotifier` | L44 | private |
| struct | `BufferedTimeline` | L57 | private |
| struct | `ContinuousResampler` | L266 | private |

## Symbols & Routines (25)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `empty_queue_action` | L22 | `fn empty_queue_action(end_of_stream: bool, pending_samples: usize) -> EmptyQueueAction` |
| function | `drop` | L47 | `fn drop(&mut self) -> ()` |
| function | `new` | L64 | `fn new() -> Self` |
| function | `available` | L71 | `fn available(&self) -> usize` |
| function | `append` | L76 | `fn append(&mut self, samples: Vec<f32>) -> ()` |
| function | `take` | L79 | `fn take(&mut self, count: usize) -> Vec<f32>` |
| function | `seek_seconds` | L92 | `fn seek_seconds(&mut self, seconds: i32, sample_rate: u32) -> ()` |
| function | `rewind_to` | L102 | `fn rewind_to(&mut self, checkpoint: u64) -> ()` |
| function | `prune_played_history` | L106 | `fn prune_played_history(&mut self, sample_rate: u32) -> ()` |
| function | `feed` | L145 | `fn feed(&self, samples: Vec<f32>) -> ()` |
| function | `end_of_stream` | L152 | `fn end_of_stream(&self) -> ()` |
| function | `pause` | L156 | `fn pause(&self) -> ()` |
| function | `resume` | L160 | `fn resume(&self) -> ()` |
| function | `seek_seconds` | L164 | `fn seek_seconds(&self, seconds: i32) -> ()` |
| function | `set_speed` | L168 | `fn set_speed(&self, speed: f32) -> ()` |
| function | `stop` | L172 | `fn stop(mut self) -> ()` |
| function | `drop` | L181 | `fn drop(&mut self) -> ()` |
| function | `drain_feed` | L189 | `fn drain_feed(feed_rx: &Receiver<Vec<f32>>, timeline: &mut BufferedTimeline) -> ()` |
| function | `drain_controls` | L196 | `fn drain_controls(control_rx: &Receiver<PlaybackControl>, sample_rate: u32, timeline: &mut BufferedTimeline, output: &mut VecDeque<f32>, output_checkpoint: &mut Option<u64>, speed: &mut f32, stretcher: &mut Option<suflyor_wsola::StreamingWsola>, stretch_finished: &mut bool,) -> ()` |
| function | `fill_output` | L225 | `fn fill_output(sample_rate: u32, speed: f32, eos: bool, timeline: &mut BufferedTimeline, output: &mut VecDeque<f32>, output_checkpoint: &mut Option<u64>, stretcher: &mut Option<suflyor_wsola::StreamingWsola>, stretch_finished: &mut bool,) -> ()` |
| function | `new` | L274 | `fn new(src_rate: u32, dst_rate: u32) -> Self` |
| function | `process` | L283 | `fn process(&mut self, input: &[f32], output: &mut VecDeque<f32>) -> ()` |
| function | `render_loop` | L322 | `fn render_loop(sample_rate: u32, feed_rx: Receiver<Vec<f32>>, control_rx: Receiver<PlaybackControl>, eos: Arc<AtomicBool>, stop: Arc<AtomicBool>, paused: Arc<AtomicBool>,) -> Result<()>` |
| function | `end_of_stream_drains_without_refilling_silence` | L457 | `fn end_of_stream_drains_without_refilling_silence() -> ()` |
| function | `exit_notifier_runs_when_its_scope_ends` | L464 | `fn exit_notifier_runs_when_its_scope_ends() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L118
- Instantiates IPC channel at L119
