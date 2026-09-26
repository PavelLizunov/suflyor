---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3096c03234cb"
source_path: "overlay-backend/src/tts.rs"
batch_id: "B10"
total_lines: 2221
symbols_count: 133
review_state: validated
---

# File Map: `overlay-backend/src/tts.rs`

- **Batch:** B10
- **Physical Lines:** 2221
- **Coverage:** 2221/2221 lines (100%)

## Types & Structures (10)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `EngineKind` | L27 | pub |
| enum | `PlaybackEvent` | L415 | private |
| struct | `VoiceRef` | L49 | pub |
| struct | `TeraReady` | L88 | pub |
| struct | `SpeakingState` | L141 | private |
| struct | `PlaybackTracker` | L484 | private |
| struct | `VoiceInfo` | L830 | pub |
| struct | `drives` | L840 | private |
| struct | `Sidecar` | L841 | private |
| struct | `Tts` | L1059 | pub |

## Symbols & Routines (133)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `parse_engine` | L37 | `fn parse_engine(raw: &str) -> EngineKind` |
| function | `parse_voice_ref` | L55 | `fn parse_voice_ref(raw: &str) -> VoiceRef` |
| function | `format_voice_ref` | L76 | `fn format_voice_ref(voice: &VoiceRef) -> String` |
| function | `parse_ready_line` | L99 | `fn parse_ready_line(line: &str) -> Option<TeraReady>` |
| function | `playback_speed_percent` | L170 | `fn playback_speed_percent(speed: f32) -> u16` |
| function | `extend_speaking_for_slower_playback` | L174 | `fn extend_speaking_for_slower_playback(old_percent: u16, new_percent: u16) -> ()` |
| function | `rewind_extension_ms` | L196 | `fn rewind_extension_ms(seconds: i32, speed_percent: u16) -> u64` |
| function | `extend_speaking_for_rewind` | L205 | `fn extend_speaking_for_rewind(seconds: i32, speed_percent: u16) -> ()` |
| function | `is_speaking` | L224 | `fn is_speaking() -> bool` |
| function | `is_loading` | L235 | `fn is_loading() -> bool` |
| function | `should_suppress_stt` | L245 | `fn should_suppress_stt() -> bool` |
| function | `rate_to_speed` | L257 | `fn rate_to_speed(rate: i32) -> f64` |
| function | `mark_speaking_for` | L269 | `fn mark_speaking_for(chars: usize) -> u64` |
| function | `mark_loading_generation` | L292 | `fn mark_loading_generation(generation: u64) -> ()` |
| function | `terminal_watchdog_until` | L304 | `fn terminal_watchdog_until(now_ms: u64, estimated_until_ms: u64) -> u64` |
| function | `arm_terminal_watchdog` | L313 | `fn arm_terminal_watchdog(generation: u64) -> bool` |
| function | `clear_loading_generation` | L325 | `fn clear_loading_generation(generation: u64) -> bool` |
| function | `clear_speaking` | L337 | `fn clear_speaking() -> ()` |
| function | `clear_speaking_generation` | L352 | `fn clear_speaking_generation(generation: u64) -> bool` |
| function | `active_speaking_generation` | L369 | `fn active_speaking_generation() -> Option<u64>` |
| function | `paused_remaining` | L378 | `fn paused_remaining(until_ms: u64, now_ms: u64) -> Option<u64>` |
| function | `resumed_until` | L382 | `fn resumed_until(remaining_ms: u64, now_ms: u64) -> Option<u64>` |
| function | `pause_speaking` | L386 | `fn pause_speaking() -> ()` |
| function | `resume_speaking` | L402 | `fn resume_speaking() -> ()` |
| function | `parse_playback_event` | L425 | `fn parse_playback_event(line: &str) -> Result<Option<PlaybackEvent>, &'static str>` |
| function | `register_speak` | L490 | `fn register_speak(&mut self, generation: u64) -> ()` |
| function | `cancel_speak` | L494 | `fn cancel_speak(&mut self, generation: u64) -> bool` |
| function | `started` | L501 | `fn started(&mut self, id: u64) -> Option<u64>` |
| function | `terminal` | L507 | `fn terminal(&mut self, id: u64) -> Option<u64>` |
| function | `generation` | L511 | `fn generation(&self, id: u64) -> Option<u64>` |
| function | `stop_generation` | L517 | `fn stop_generation(&mut self, generation: u64) -> bool` |
| function | `rejected` | L533 | `fn rejected(&mut self, reason: &str) -> Option<u64>` |
| function | `drain_generations` | L540 | `fn drain_generations(&mut self) -> Vec<u64>` |
| function | `finish_generation` | L547 | `fn finish_generation(engine: EngineKind, generation: Option<u64>, event: &str, sidecar_id: Option<u64>,) -> ()` |
| function | `handle_playback_line` | L563 | `fn handle_playback_line(engine: EngineKind, playback: &Mutex<PlaybackTracker>, line: &str) -> ()` |
| function | `handle_playback_eof` | L629 | `fn handle_playback_eof(engine: EngineKind, playback: &Mutex<PlaybackTracker>) -> ()` |
| function | `to_speech` | L650 | `fn to_speech(md: &str) -> String` |
| function | `is_rule` | L679 | `fn is_rule(t: &str) -> bool` |
| function | `is_table_separator` | L687 | `fn is_table_separator(t: &str) -> bool` |
| function | `strip_bullet` | L692 | `fn strip_bullet(line: &str) -> &str` |
| function | `strip_inline` | L717 | `fn strip_inline(s: &str) -> String` |
| function | `strip_links` | L726 | `fn strip_links(s: &str) -> String` |
| function | `parse_link` | L751 | `fn parse_link(b: &[char], open: usize) -> Option<(String, usize)>` |
| function | `normalize_ws` | L763 | `fn normalize_ws(s: &str) -> String` |
| function | `strips_emphasis_and_code` | L787 | `fn strips_emphasis_and_code() -> ()` |
| function | `strips_headings_bullets_links` | L796 | `fn strips_headings_bullets_links() -> ()` |
| function | `drops_table_separator_and_rule` | L809 | `fn drops_table_separator_and_rule() -> ()` |
| function | `keeps_plain_text_and_underscores` | L817 | `fn keeps_plain_text_and_underscores() -> ()` |
| function | `ensure` | L869 | `fn ensure(&mut self) -> ()` |
| function | `crashed_out` | L942 | `fn crashed_out(&self) -> bool` |
| function | `write_raw` | L950 | `fn write_raw(&mut self, line: &str) -> bool` |
| function | `send` | L962 | `fn send(&mut self, line: &str) -> bool` |
| function | `send_tera_speak` | L969 | `fn send_tera_speak(&mut self, line: &str, chars: usize) -> bool` |
| function | `send_piper_speak` | L996 | `fn send_piper_speak(&mut self, line: &str, chars: usize) -> bool` |
| function | `send_if_alive` | L1026 | `fn send_if_alive(&mut self, line: &str) -> bool` |
| function | `engine_code` | L1040 | `fn engine_code(kind: EngineKind) -> u8` |
| function | `engine_from_code` | L1047 | `fn engine_from_code(code: u8) -> EngineKind` |
| function | `spawn` | L1076 | `fn spawn(engine: EngineKind, voice_raw: Option<String>, rate: i32, lang: &str) -> Self` |
| function | `engine_kind` | L1129 | `fn engine_kind(&self) -> EngineKind` |
| function | `set_engine` | L1135 | `fn set_engine(&self, kind: EngineKind) -> ()` |
| function | `tera_usable` | L1143 | `fn tera_usable(&self) -> bool` |
| function | `tera_ready` | L1157 | `fn tera_ready(&self) -> Option<TeraReady>` |
| function | `is_available` | L1167 | `fn is_available(&self) -> bool` |
| function | `voices` | L1176 | `fn voices(&self) -> &[VoiceInfo]` |
| function | `tera_voices` | L1183 | `fn tera_voices() -> Vec<String>` |
| function | `send_to` | L1204 | `fn send_to(&self, target: u8, line: &str) -> bool` |
| function | `send_tera_speak` | L1212 | `fn send_tera_speak(&self, line: &str, chars: usize) -> bool` |
| function | `send_piper_speak` | L1219 | `fn send_piper_speak(&self, line: &str, chars: usize) -> bool` |
| function | `control_to` | L1228 | `fn control_to(&self, target: u8, line: &str) -> bool` |
| function | `speak` | L1251 | `fn speak(&self, text: &str) -> bool` |
| function | `pause` | L1277 | `fn pause(&self) -> ()` |
| function | `resume` | L1286 | `fn resume(&self) -> ()` |
| function | `stop` | L1293 | `fn stop(&self) -> ()` |
| function | `set_rate` | L1350 | `fn set_rate(&self, rate: i32) -> ()` |
| function | `set_playback_speed` | L1366 | `fn set_playback_speed(&self, speed: f32) -> ()` |
| function | `seek_seconds` | L1382 | `fn seek_seconds(&self, seconds: i32) -> ()` |
| function | `set_voice` | L1391 | `fn set_voice(&self, id: &str) -> ()` |
| function | `warm` | L1407 | `fn warm(&self) -> ()` |
| function | `init` | L1437 | `fn init(engine: Option<String>, voice_id: Option<String>, rate: i32, lang: &str) -> ()` |
| function | `with` | L1448 | `fn with(f: impl FnOnce(&Tts) -> R) -> Option<R>` |
| function | `speak` | L1458 | `fn speak(text: &str) -> bool` |
| function | `pause` | L1461 | `fn pause() -> ()` |
| function | `resume` | L1464 | `fn resume() -> ()` |
| function | `stop` | L1467 | `fn stop() -> ()` |
| function | `set_rate` | L1470 | `fn set_rate(rate: i32) -> ()` |
| function | `set_playback_speed` | L1474 | `fn set_playback_speed(speed: f32) -> ()` |
| function | `seek_seconds` | L1478 | `fn seek_seconds(seconds: i32) -> ()` |
| function | `set_voice` | L1481 | `fn set_voice(id: &str) -> ()` |
| function | `set_engine` | L1485 | `fn set_engine(engine_raw: &str) -> ()` |
| function | `active_engine` | L1491 | `fn active_engine() -> EngineKind` |
| function | `tera_ready` | L1496 | `fn tera_ready() -> Option<TeraReady>` |
| function | `tera_usable` | L1502 | `fn tera_usable() -> bool` |
| function | `tera_voice_ids` | L1507 | `fn tera_voice_ids() -> Vec<String>` |
| function | `warm` | L1511 | `fn warm() -> ()` |
| function | `voices` | L1517 | `fn voices(ru: bool) -> Vec<VoiceInfo>` |
| function | `is_available` | L1526 | `fn is_available() -> bool` |
| function | `available_on_disk` | L1530 | `fn available_on_disk() -> bool` |
| function | `sidecar_exe_path` | L1541 | `fn sidecar_exe_path() -> PathBuf` |
| function | `tera_sidecar_exe_path` | L1551 | `fn tera_sidecar_exe_path() -> PathBuf` |
| function | `sidecar_paths_use_the_platform_executable_suffix` | L1561 | `fn sidecar_paths_use_the_platform_executable_suffix() -> ()` |
| function | `sidecar_stderr` | L1577 | `fn sidecar_stderr(kind: EngineKind) -> Stdio` |
| function | `spawn_engine_sidecar` | L1594 | `fn spawn_engine_sidecar(exe: &Path, kind: EngineKind) -> std::io::Result<Proc>` |
| function | `tts_root` | L1606 | `fn tts_root() -> Option<PathBuf>` |
| function | `scan_installed_voices` | L1612 | `fn scan_installed_voices(ru: bool) -> Vec<VoiceInfo>` |
| function | `has_onnx` | L1639 | `fn has_onnx(dir: &Path) -> bool` |
| function | `pick_voice_id` | L1654 | `fn pick_voice_id(voices: &[VoiceInfo], configured: &str) -> Option<String>` |
| function | `friendly_name` | L1669 | `fn friendly_name(dir: &str, ru: bool) -> String` |
| function | `pause_and_resume_preserve_a_finite_deadline` | L1713 | `fn pause_and_resume_preserve_a_finite_deadline() -> ()` |
| function | `rate_is_clamped` | L1722 | `fn rate_is_clamped() -> ()` |
| function | `friendly_name_maps_known_voices` | L1728 | `fn friendly_name_maps_known_voices() -> ()` |
| function | `pick_voice_prefers_irina_then_first` | L1753 | `fn pick_voice_prefers_irina_then_first() -> ()` |
| function | `speak_encodes_base64` | L1776 | `fn speak_encodes_base64() -> ()` |
| function | `playback_speed_is_protocol_bounded` | L1788 | `fn playback_speed_is_protocol_bounded() -> ()` |
| function | `engine_selection_defaults_to_piper` | L1802 | `fn engine_selection_defaults_to_piper() -> ()` |
| function | `voice_refs_namespace_and_legacy_compat` | L1811 | `fn voice_refs_namespace_and_legacy_compat() -> ()` |
| function | `ready_handshake_parses_capabilities` | L1842 | `fn ready_handshake_parses_capabilities() -> ()` |
| function | `ready_handshake_rejects_foreign_lines` | L1860 | `fn ready_handshake_rejects_foreign_lines() -> ()` |
| function | `tera_playback_events_parse_strictly` | L1874 | `fn tera_playback_events_parse_strictly() -> ()` |
| function | `tera_done_clears_matching_suppression_immediately` | L1913 | `fn tera_done_clears_matching_suppression_immediately() -> ()` |
| function | `piper_keeps_speaking_until_its_real_done_event` | L1934 | `fn piper_keeps_speaking_until_its_real_done_event() -> ()` |
| function | `terminal_watchdog_is_generous_but_finite` | L1956 | `fn terminal_watchdog_is_generous_but_finite() -> ()` |
| function | `missing_terminal_event_eventually_releases_player_and_stt` | L1963 | `fn missing_terminal_event_eventually_releases_player_and_stt() -> ()` |
| function | `stale_tera_terminal_cannot_clear_newer_utterance` | L1980 | `fn stale_tera_terminal_cannot_clear_newer_utterance() -> ()` |
| function | `explicit_tera_stop_forgets_only_its_generation` | L2000 | `fn explicit_tera_stop_forgets_only_its_generation() -> ()` |
| function | `pause_keeps_ui_active_but_suppresses_only_the_drain_tail` | L2051 | `fn pause_keeps_ui_active_but_suppresses_only_the_drain_tail() -> ()` |
| function | `tera_failed_rejected_and_eof_clear_only_tracked_speech` | L2090 | `fn tera_failed_rejected_and_eof_clear_only_tracked_speech() -> ()` |
| function | `piper_without_terminal_event_keeps_estimate_fallback` | L2121 | `fn piper_without_terminal_event_keeps_estimate_fallback() -> ()` |
| function | `tera_engine_falls_back_when_sidecar_missing` | L2130 | `fn tera_engine_falls_back_when_sidecar_missing() -> ()` |
| function | `missing_exe_sidecar` | L2147 | `fn missing_exe_sidecar(kind: EngineKind) -> Sidecar` |
| function | `send_reports_failure_when_the_sidecar_cannot_run` | L2168 | `fn send_reports_failure_when_the_sidecar_cannot_run() -> ()` |
| function | `control_commands_never_respawn_a_dead_sidecar` | L2179 | `fn control_commands_never_respawn_a_dead_sidecar() -> ()` |
| function | `stop_clears_speaking_even_when_no_sidecar_runs` | L2191 | `fn stop_clears_speaking_even_when_no_sidecar_runs() -> ()` |
| function | `crash_limit_bypasses_the_engine` | L2203 | `fn crash_limit_bypasses_the_engine() -> ()` |
