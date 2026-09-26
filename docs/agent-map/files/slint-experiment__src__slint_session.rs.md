---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_68cf2ea4a0ae"
source_path: "slint-experiment/src/slint_session.rs"
batch_id: "B01"
total_lines: 2117
symbols_count: 48
review_state: validated
---

# File Map: `slint-experiment/src/slint_session.rs`

- **Batch:** B01
- **Physical Lines:** 2117
- **Coverage:** 2117/2117 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `AutoTilePermit` | L101 | private |
| struct | `SystemAudioAuxGuard` | L136 | pub |
| struct | `SystemAudioSessionStartGuard` | L162 | private |

## Symbols & Routines (48)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `auto_tile_max_tokens` | L45 | `fn auto_tile_max_tokens(is_mlx: bool) -> u32` |
| function | `auto_tile_prompt_hash` | L54 | `fn auto_tile_prompt_hash(effective_context: &str, recent_transcript: &[String], live_coaching: bool, is_mlx: bool,) -> u64` |
| function | `auto_tile_single_flight_required` | L71 | `fn auto_tile_single_flight_required(is_mlx: bool, every_line: bool) -> bool` |
| function | `auto_tile_trigger` | L76 | `fn auto_tile_trigger(text: &str, every_line: bool, trigger_keywords: &str,) -> Option<backend_runtime::Trigger>` |
| function | `drop` | L107 | `fn drop(&mut self) -> ()` |
| function | `try_acquire_auto_tile` | L118 | `fn try_acquire_auto_tile(state: &AtomicU64, session_gen: u64) -> Option<AutoTilePermit<'_>>` |
| function | `drop` | L139 | `fn drop(&mut self) -> ()` |
| function | `try_acquire_system_audio_aux` | L150 | `fn try_acquire_system_audio_aux() -> Option<SystemAudioAuxGuard>` |
| function | `acquire` | L167 | `fn acquire() -> Result<Self>` |
| function | `disarm` | L182 | `fn disarm(&mut self) -> ()` |
| function | `drop` | L188 | `fn drop(&mut self) -> ()` |
| function | `release_system_audio_session` | L195 | `fn release_system_audio_session() -> ()` |
| function | `begin_system_audio_collector` | L211 | `fn begin_system_audio_collector(rt: &SharedSlintRuntime) -> bool` |
| function | `finish_system_audio_collector` | L222 | `fn finish_system_audio_collector(rt: &SharedSlintRuntime) -> Option<Vec<i16>>` |
| function | `collect_system_audio` | L226 | `fn collect_system_audio(state: &mut SlintRuntime, chunk: &AudioChunk) -> ()` |
| function | `start_session` | L246 | `fn start_session(events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, rt: SharedSlintRuntime,) -> Result<()>` |
| function | `start_session_with_recovery` | L264 | `fn start_session_with_recovery(events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, rt: SharedSlintRuntime, recovered_from: String,) -> Result<()>` |
| function | `start_session_inner` | L273 | `fn start_session_inner(events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, rt: SharedSlintRuntime, recovered_from: Option<String>,) -> Result<()>` |
| function | `forward_audio_chunks` | L598 | `fn forward_audio_chunks(mut src_rx: tokio::sync::mpsc::Receiver<AudioChunk>, rt: SharedSlintRuntime, recorder: Option<SessionRecorder>,) -> tokio::sync::mpsc::Receiver<AudioChunk>` |
| function | `transcript_forwarder` | L677 | `fn transcript_forwarder(mut stt_rx: tokio::sync::mpsc::Receiver<stt::TranscriptEvent>, events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, rt: SharedSlintRuntime, journal: Journal,) -> ()` |
| function | `detector_allows` | L784 | `fn detector_allows(source: AudioSource, skip_mic: bool) -> bool` |
| function | `trigger_highlights` | L796 | `fn trigger_highlights(trigger: &backend_runtime::Trigger) -> Vec<String>` |
| function | `mic_notice_decision` | L834 | `fn mic_notice_decision(mic_state: &str, latch: bool) -> (bool, bool)` |
| function | `mic_down_notice` | L842 | `fn mic_down_notice(ui_is_ru: bool) -> (&'static str, &'static str)` |
| function | `maybe_spawn_auto_tile` | L869 | `fn maybe_spawn_auto_tile(events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, rt: SharedSlintRuntime, journal: Journal, text: String, session_gen: u64,) -> ()` |
| function | `stop_session` | L1396 | `fn stop_session(rt: SharedSlintRuntime, cfg: &SharedConfig) -> Vec<TranscriptLine>` |
| function | `debrief_gate` | L1550 | `fn debrief_gate(cfg: &SharedConfig, transcript: &[TranscriptLine], session_duration_ms: u64,) -> Result<(), &'static str>` |
| function | `maybe_run_debrief` | L1585 | `fn maybe_run_debrief(events: Arc<dyn RuntimeEvents>, cfg: SharedConfig, transcript: Vec<TranscriptLine>, session_id: String, session_duration_ms: u64, rt_handle: &tokio::runtime::Handle,) -> ()` |
| function | `meeting_ending_phrase_match` | L1645 | `fn meeting_ending_phrase_match(text: &str) -> bool` |
| function | `log_warn` | L1690 | `fn log_warn(msg: &str) -> ()` |
| function | `log_info` | L1693 | `fn log_info(msg: &str) -> ()` |
| function | `auto_tile_permit_is_single_flight_per_session` | L1708 | `fn auto_tile_permit_is_single_flight_per_session() -> ()` |
| function | `auto_tile_cache_hash_tracks_every_prompt_input` | L1725 | `fn auto_tile_cache_hash_tracks_every_prompt_input() -> ()` |
| function | `auto_tile_policy_and_token_selection_follow_mlx_rules` | L1748 | `fn auto_tile_policy_and_token_selection_follow_mlx_rules() -> ()` |
| function | `aggressive_auto_tile_uses_question_suffix_and_filters_noise` | L1759 | `fn aggressive_auto_tile_uses_question_suffix_and_filters_noise() -> ()` |
| function | `normal_auto_tile_mode_still_delegates_to_detector` | L1784 | `fn normal_auto_tile_mode_still_delegates_to_detector() -> ()` |
| function | `system_audio_owner_is_atomic_between_session_and_auxiliary_capture` | L1796 | `fn system_audio_owner_is_atomic_between_session_and_auxiliary_capture() -> ()` |
| function | `session_restart_releases_prior_owner_and_reacquires` | L1815 | `fn session_restart_releases_prior_owner_and_reacquires() -> ()` |
| function | `audio_forwarder_gates_stt_while_paused_without_recording` | L1839 | `fn audio_forwarder_gates_stt_while_paused_without_recording() -> ()` |
| function | `audio_forwarder_drops_muted_mic_before_stt` | L1877 | `fn audio_forwarder_drops_muted_mic_before_stt() -> ()` |
| function | `audio_forwarder_keeps_muted_mic_out_of_recording` | L1940 | `fn audio_forwarder_keeps_muted_mic_out_of_recording() -> ()` |
| function | `mic_notice_fires_once_per_down_episode` | L1991 | `fn mic_notice_fires_once_per_down_episode() -> ()` |
| function | `mic_down_notice_follows_ui_language` | L2008 | `fn mic_down_notice_follows_ui_language() -> ()` |
| function | `meeting_ending_detects_canonical_patterns` | L2016 | `fn meeting_ending_detects_canonical_patterns() -> ()` |
| function | `meeting_ending_ignores_mid_interview_thanks` | L2031 | `fn meeting_ending_ignores_mid_interview_thanks() -> ()` |
| function | `session_system_collector_is_source_filtered_and_bounded` | L2040 | `fn session_system_collector_is_source_filtered_and_bounded() -> ()` |
| function | `stop_session_on_empty_rt_returns_empty_snapshot` | L2078 | `fn stop_session_on_empty_rt_returns_empty_snapshot() -> ()` |
| function | `stop_session_keeps_full_transcript_for_summary` | L2090 | `fn stop_session_keeps_full_transcript_for_summary() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L241
- Spawns asynchronous thread/task at L526
- Spawns asynchronous thread/task at L580
- Instantiates IPC channel at L603
- Spawns asynchronous thread/task at L604
- Spawns asynchronous thread/task at L762
- Spawns asynchronous thread/task at L1487
