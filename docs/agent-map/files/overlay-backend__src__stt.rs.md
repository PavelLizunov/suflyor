---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_9f4acc902ca9"
source_path: "overlay-backend/src/stt.rs"
batch_id: "B07"
total_lines: 1643
symbols_count: 62
review_state: validated
---

# File Map: `overlay-backend/src/stt.rs`

- **Batch:** B07
- **Physical Lines:** 1643
- **Coverage:** 1643/1643 lines (100%)

## Types & Structures (6)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `Model` | L34 | pub(super) |
| struct | `Model` | L87 | pub(super) |
| struct | `TranscriptEvent` | L269 | pub |
| struct | `GroqResponse` | L276 | private |
| struct | `Utterance` | L589 | private |
| struct | `an` | L1362 | private |

## Symbols & Routines (62)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `configure_accelerator` | L36 | `fn configure_accelerator(use_gpu: bool) -> ()` |
| function | `load` | L55 | `fn load(dir: &str) -> anyhow::Result<Model>` |
| function | `transcribe_f32` | L74 | `fn transcribe_f32(&mut self, samples: &[f32]) -> anyhow::Result<String>` |
| function | `configure_accelerator` | L89 | `fn configure_accelerator(_use_gpu: bool) -> ()` |
| function | `load` | L96 | `fn load(_dir: &str) -> anyhow::Result<Model>` |
| function | `transcribe_f32` | L101 | `fn transcribe_f32(&mut self, _samples: &[f32]) -> anyhow::Result<String>` |
| function | `unsupported` | L106 | `fn unsupported() -> anyhow::Error` |
| function | `public_gigaam_load_error` | L111 | `fn public_gigaam_load_error(error: anyhow::Error, public_message: &'static str) -> anyhow::Error` |
| function | `configure_gigaam_accelerator` | L122 | `fn configure_gigaam_accelerator(use_gpu: bool) -> ()` |
| function | `test_connection` | L129 | `fn test_connection(api_key: String) -> Result<String>` |
| function | `test_connection_backend` | L154 | `fn test_connection_backend(backend: &SttBackendCfg) -> Result<String>` |
| function | `validate_gigaam_dir` | L217 | `fn validate_gigaam_dir(model_dir: &str) -> Result<()>` |
| function | `utterance_cap_sec` | L237 | `fn utterance_cap_sec(source: AudioSource) -> u64` |
| function | `utterance_flush_decision` | L244 | `fn utterance_flush_decision(duration_sec: f32, silent_run_ms: u64, had_voice: bool, cap_sec: u64,) -> (bool, bool)` |
| function | `spawn` | L289 | `fn spawn(mut audio_rx: mpsc::Receiver<AudioChunk>, backend: SttBackendCfg, language: Option<String>, whisper_prompt: Option<String>, health: std::sync::Arc<crate::health::HealthSignals>,) -> mpsc::Receiver<TranscriptEvent>` |
| function | `buffer_likely_speech` | L605 | `fn buffer_likely_speech(samples: &[i16]) -> bool` |
| function | `is_likely_hallucination` | L653 | `fn is_likely_hallucination(text: &str) -> bool` |
| function | `rms_i16` | L732 | `fn rms_i16(samples: &[i16]) -> f32` |
| function | `shared_gigaam_model` | L748 | `fn shared_gigaam_model(model_dir: &str) -> Result<SharedGigaamModel>` |
| function | `reset_gigaam_cache` | L766 | `fn reset_gigaam_cache() -> ()` |
| function | `transcribe_once` | L776 | `fn transcribe_once(backend: &SttBackendCfg, pcm: &[i16], language: Option<&str>, whisper_prompt: Option<&str>,) -> Result<String>` |
| function | `transcribe` | L829 | `fn transcribe(client: &reqwest::Client, url: &str, bearer: Option<&str>, pcm: &[i16], language: Option<&str>, prompt: Option<&str>, stt_model: &str,) -> Result<String>` |
| function | `is_permanent_error` | L872 | `fn is_permanent_error(msg: &str) -> bool` |
| function | `transcribe_once_attempt` | L880 | `fn transcribe_once_attempt(client: &reqwest::Client, url: &str, bearer: Option<&str>, wav: &[u8], language: Option<&str>, prompt: Option<&str>, stt_model: &str,) -> Result<String>` |
| function | `finish_transcript` | L960 | `fn finish_transcript(result: Result<String>, src: AudioSource, start_ts_ms: u64, sample_count: usize, tx: &mpsc::Sender<TranscriptEvent>, health: &crate::health::HealthSignals,) -> ()` |
| function | `gigaam_transcribe` | L1008 | `fn gigaam_transcribe(model: &Arc<Mutex<gigaam_shim::Model>>, pcm: &[i16]) -> Result<String>` |
| function | `build_whisper_prompt` | L1059 | `fn build_whisper_prompt(keywords: &str, meeting_context: &str) -> Option<String>` |
| function | `encode_wav_pcm_i16_mono_16k` | L1158 | `fn encode_wav_pcm_i16_mono_16k(pcm: &[i16]) -> Result<Vec<u8>>` |
| function | `live_utterance_flushes_at_per_source_caps_or_four_silent_chunks` | L1193 | `fn live_utterance_flushes_at_per_source_caps_or_four_silent_chunks() -> ()` |
| function | `permanent_error_classification_survives_redaction` | L1233 | `fn permanent_error_classification_survives_redaction() -> ()` |
| function | `wav_header_is_44_bytes` | L1249 | `fn wav_header_is_44_bytes() -> ()` |
| function | `wav_roundtrip_through_hound_preserves_samples_and_format` | L1263 | `fn wav_roundtrip_through_hound_preserves_samples_and_format() -> ()` |
| function | `rms_of_zeros_is_zero` | L1291 | `fn rms_of_zeros_is_zero() -> ()` |
| function | `rms_of_const_is_const` | L1296 | `fn rms_of_const_is_const() -> ()` |
| function | `rms_handles_empty_input_without_div_by_zero` | L1303 | `fn rms_handles_empty_input_without_div_by_zero() -> ()` |
| function | `rms_ignores_sign_via_squaring` | L1309 | `fn rms_ignores_sign_via_squaring() -> ()` |
| function | `rms_max_amplitude_does_not_overflow_or_nan` | L1317 | `fn rms_max_amplitude_does_not_overflow_or_nan() -> ()` |
| function | `wav_with_empty_pcm_produces_valid_header_only` | L1329 | `fn wav_with_empty_pcm_produces_valid_header_only() -> ()` |
| function | `wav_riff_size_matches_actual_content_length` | L1339 | `fn wav_riff_size_matches_actual_content_length() -> ()` |
| function | `permanent_errors_short_circuit_retry` | L1352 | `fn permanent_errors_short_circuit_retry() -> ()` |
| function | `stt_transport_failure_kind_does_not_leak_urls` | L1361 | `fn stt_transport_failure_kind_does_not_leak_urls() -> ()` |
| function | `whisper_prompt_returns_none_for_empty_inputs` | L1392 | `fn whisper_prompt_returns_none_for_empty_inputs() -> ()` |
| function | `whisper_prompt_never_exceeds_groq_hard_limit` | L1404 | `fn whisper_prompt_never_exceeds_groq_hard_limit() -> ()` |
| function | `whisper_prompt_includes_keywords_for_bias` | L1426 | `fn whisper_prompt_includes_keywords_for_bias() -> ()` |
| function | `whisper_prompt_leads_with_canonical_tech_vocab` | L1435 | `fn whisper_prompt_leads_with_canonical_tech_vocab() -> ()` |
| function | `whisper_prompt_includes_context_snippet_after_keywords` | L1460 | `fn whisper_prompt_includes_context_snippet_after_keywords() -> ()` |
| function | `whisper_prompt_caps_at_max_chars_for_token_budget` | L1471 | `fn whisper_prompt_caps_at_max_chars_for_token_budget() -> ()` |
| function | `whisper_prompt_keywords_only_no_context_section` | L1486 | `fn whisper_prompt_keywords_only_no_context_section() -> ()` |
| function | `whisper_prompt_context_only_no_terms_section` | L1495 | `fn whisper_prompt_context_only_no_terms_section() -> ()` |
| function | `whisper_prompt_skips_context_when_budget_exhausted` | L1505 | `fn whisper_prompt_skips_context_when_budget_exhausted() -> ()` |
| function | `noise_gate_rejects_pure_silence` | L1522 | `fn noise_gate_rejects_pure_silence() -> ()` |
| function | `noise_gate_rejects_low_level_noise` | L1528 | `fn noise_gate_rejects_low_level_noise() -> ()` |
| function | `noise_gate_rejects_silence_plus_one_spike` | L1535 | `fn noise_gate_rejects_silence_plus_one_spike() -> ()` |
| function | `noise_gate_accepts_sustained_speech` | L1546 | `fn noise_gate_accepts_sustained_speech() -> ()` |
| function | `hallucination_filter_drops_empty_or_punct` | L1558 | `fn hallucination_filter_drops_empty_or_punct() -> ()` |
| function | `hallucination_filter_drops_known_phrases` | L1566 | `fn hallucination_filter_drops_known_phrases() -> ()` |
| function | `hallucination_filter_drops_repetition_loop` | L1575 | `fn hallucination_filter_drops_repetition_loop() -> ()` |
| function | `hallucination_filter_accepts_real_speech` | L1585 | `fn hallucination_filter_accepts_real_speech() -> ()` |
| function | `transient_errors_keep_retrying` | L1601 | `fn transient_errors_keep_retrying() -> ()` |
| function | `gigaam_shim_load_unsupported_off_windows` | L1620 | `fn gigaam_shim_load_unsupported_off_windows() -> ()` |
| function | `validate_gigaam_dir_unsupported_off_windows` | L1632 | `fn validate_gigaam_dir_unsupported_off_windows() -> ()` |
| function | `configure_gigaam_accelerator_honest_noop_off_windows` | L1638 | `fn configure_gigaam_accelerator_honest_noop_off_windows() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L296
- Spawns asynchronous thread/task at L310
- Spawns asynchronous thread/task at L513
- Spawns asynchronous thread/task at L552
