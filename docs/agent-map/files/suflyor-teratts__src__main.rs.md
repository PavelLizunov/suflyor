---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_89959d88732d"
source_path: "suflyor-teratts/src/main.rs"
batch_id: "B12"
total_lines: 1113
symbols_count: 62
review_state: validated
---

# File Map: `suflyor-teratts/src/main.rs`

- **Batch:** B12
- **Physical Lines:** 1113
- **Coverage:** 1113/1113 lines (100%)

## Types & Structures (13)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Message` | L80 | private |
| struct | `SynthJob` | L92 | private |
| struct | `SynthOutcome` | L104 | private |
| struct | `Controller` | L165 | private |
| struct | `RealPlayer` | L408 | private |
| struct | `ThreadDispatch` | L435 | private |
| struct | `FailingDispatch` | L448 | private |
| struct | `FakePlayer` | L675 | private |
| struct | `RecordingDispatch` | L720 | private |
| struct | `ClosedDispatch` | L724 | private |
| struct | `Harness` | L739 | private |
| trait | `Player` | L112 | private |
| trait | `SynthDispatch` | L124 | private |

## Symbols & Routines (62)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `tts_root` | L57 | `fn tts_root() -> Option<PathBuf>` |
| function | `emit` | L73 | `fn emit(out: &mut impl Write, event: &Event) -> ()` |
| function | `feed` | L113 | `fn feed(&mut self, samples: Vec<f32>) -> ()` |
| function | `end_of_stream` | L114 | `fn end_of_stream(&mut self) -> ()` |
| function | `pause` | L115 | `fn pause(&mut self) -> ()` |
| function | `resume` | L116 | `fn resume(&mut self) -> ()` |
| function | `seek_seconds` | L117 | `fn seek_seconds(&mut self, seconds: i32) -> ()` |
| function | `set_speed` | L118 | `fn set_speed(&mut self, speed: f32) -> ()` |
| function | `stop` | L119 | `fn stop(self) -> ()` |
| function | `dispatch` | L125 | `fn dispatch(&mut self, job: SynthJob) -> Result<(), SynthJob>` |
| function | `pick_default_voice` | L130 | `fn pick_default_voice(voices: &[String]) -> Option<String>` |
| function | `reason_token` | L142 | `fn reason_token(err: &anyhow::Error, fallback: &str) -> String` |
| function | `emit_event` | L188 | `fn emit_event(&mut self, event: Event) -> ()` |
| function | `emit_failed` | L192 | `fn emit_failed(&mut self, id: u64, reason: &str) -> ()` |
| function | `duration_scale` | L200 | `fn duration_scale(&self) -> f32` |
| function | `close_active` | L210 | `fn close_active(&mut self) -> ()` |
| function | `speak` | L225 | `fn speak(&mut self, text: &str) -> ()` |
| function | `on_synth_result` | L298 | `fn on_synth_result(&mut self, outcome: SynthOutcome) -> ()` |
| function | `on_playback_done` | L350 | `fn on_playback_done(&mut self, id: u64) -> ()` |
| function | `on_cmd` | L366 | `fn on_cmd(&mut self, cmd: Cmd) -> ()` |
| function | `feed` | L411 | `fn feed(&mut self, samples: Vec<f32>) -> ()` |
| function | `end_of_stream` | L414 | `fn end_of_stream(&mut self) -> ()` |
| function | `pause` | L417 | `fn pause(&mut self) -> ()` |
| function | `resume` | L420 | `fn resume(&mut self) -> ()` |
| function | `seek_seconds` | L423 | `fn seek_seconds(&mut self, seconds: i32) -> ()` |
| function | `set_speed` | L426 | `fn set_speed(&mut self, speed: f32) -> ()` |
| function | `stop` | L429 | `fn stop(self) -> ()` |
| function | `dispatch` | L440 | `fn dispatch(&mut self, job: SynthJob) -> Result<(), SynthJob>` |
| function | `dispatch` | L453 | `fn dispatch(&mut self, job: SynthJob) -> Result<(), SynthJob>` |
| function | `synth_worker` | L466 | `fn synth_worker(mut engine: tera::TeraEngine, generation: Arc<AtomicU64>, jobs: mpsc::Receiver<SynthJob>, events: mpsc::Sender<Message>,) -> ()` |
| function | `worker` | L503 | `fn worker(mut controller: Controller<RealPlayer>, rx: mpsc::Receiver<Message>) -> ()` |
| function | `main` | L517 | `fn main() -> ()` |
| function | `feed` | L681 | `fn feed(&mut self, samples: Vec<f32>) -> ()` |
| function | `end_of_stream` | L686 | `fn end_of_stream(&mut self) -> ()` |
| function | `pause` | L691 | `fn pause(&mut self) -> ()` |
| function | `resume` | L696 | `fn resume(&mut self) -> ()` |
| function | `seek_seconds` | L701 | `fn seek_seconds(&mut self, seconds: i32) -> ()` |
| function | `set_speed` | L706 | `fn set_speed(&mut self, speed: f32) -> ()` |
| function | `stop` | L711 | `fn stop(self) -> ()` |
| function | `dispatch` | L727 | `fn dispatch(&mut self, job: SynthJob) -> Result<(), SynthJob>` |
| function | `dispatch` | L733 | `fn dispatch(&mut self, job: SynthJob) -> Result<(), SynthJob>` |
| function | `harness_with_voices` | L747 | `fn harness_with_voices(voices: Vec<String>) -> Harness` |
| function | `harness` | L787 | `fn harness() -> Harness` |
| function | `take_events` | L791 | `fn take_events(h: &Harness) -> Vec<Event>` |
| function | `ok_audio` | L795 | `fn ok_audio(utterance: u64) -> SynthOutcome` |
| function | `long_text` | L803 | `fn long_text() -> String` |
| function | `stop_during_synthesis_stops_immediately_and_discards_stale_audio` | L808 | `fn stop_during_synthesis_stops_immediately_and_discards_stale_audio() -> ()` |
| function | `newer_speak_supersedes_in_flight_synthesis` | L833 | `fn newer_speak_supersedes_in_flight_synthesis() -> ()` |
| function | `playing_is_emitted_once_when_first_audio_reaches_the_player` | L872 | `fn playing_is_emitted_once_when_first_audio_reaches_the_player() -> ()` |
| function | `every_started_gets_exactly_one_terminal_event` | L884 | `fn every_started_gets_exactly_one_terminal_event() -> ()` |
| function | `synth_failure_emits_generic_failed_and_stops_playback` | L919 | `fn synth_failure_emits_generic_failed_and_stops_playback() -> ()` |
| function | `one_bad_chunk_does_not_stop_a_long_read` | L943 | `fn one_bad_chunk_does_not_stop_a_long_read() -> ()` |
| function | `closed_worker_channel_fails_instead_of_leaving_started_active` | L967 | `fn closed_worker_channel_fails_instead_of_leaving_started_active() -> ()` |
| function | `unknown_voice_fails_without_dispatching_synthesis` | L988 | `fn unknown_voice_fails_without_dispatching_synthesis() -> ()` |
| function | `empty_text_done_without_player_or_dispatch` | L1006 | `fn empty_text_done_without_player_or_dispatch() -> ()` |
| function | `voice_command_validates_against_installed_styles` | L1018 | `fn voice_command_validates_against_installed_styles() -> ()` |
| function | `pause_resume_forward_to_the_active_player` | L1034 | `fn pause_resume_forward_to_the_active_player() -> ()` |
| function | `seek_and_speed_apply_to_the_active_and_next_generation` | L1045 | `fn seek_and_speed_apply_to_the_active_and_next_generation() -> ()` |
| function | `stale_playback_done_is_ignored` | L1061 | `fn stale_playback_done_is_ignored() -> ()` |
| function | `generation_tracks_the_active_utterance` | L1072 | `fn generation_tracks_the_active_utterance() -> ()` |
| function | `lang_and_rate_flow_into_jobs` | L1084 | `fn lang_and_rate_flow_into_jobs() -> ()` |
| function | `reason_tokens_are_protocol_safe` | L1098 | `fn reason_tokens_are_protocol_safe() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L594
- Instantiates IPC channel at L595
- Spawns asynchronous thread/task at L648
