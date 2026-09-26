---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_82c260296648"
source_path: "suflyor-tts/src/engine.rs"
batch_id: "B11"
total_lines: 305
symbols_count: 18
review_state: validated
---

# File Map: `suflyor-tts/src/engine.rs`

- **Batch:** B11
- **Physical Lines:** 305
- **Coverage:** 305/305 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `NeuralEngine` | L10 | pub |
| struct | `VoiceInfo` | L87 | pub |

## Symbols & Routines (18)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `load` | L16 | `fn load(model: &Path, tokens: &Path, data_dir: Option<&Path>) -> Result<NeuralEngine>` |
| function | `sample_rate` | L46 | `fn sample_rate(&self) -> u32` |
| function | `synth` | L51 | `fn synth(&self, text: &str, speed: f32, sid: i32) -> Result<Vec<f32>>` |
| function | `rate_to_speed` | L75 | `fn rate_to_speed(rate: i32) -> f32` |
| function | `tts_root` | L81 | `fn tts_root() -> Option<PathBuf>` |
| function | `scan_voices` | L93 | `fn scan_voices(tts_dir: &Path) -> Vec<VoiceInfo>` |
| function | `is_valid_voice_dir` | L117 | `fn is_valid_voice_dir(voice_dir: &str) -> bool` |
| function | `load_voice` | L134 | `fn load_voice(tts_dir: &Path, voice_dir: &str) -> Result<NeuralEngine>` |
| function | `find_onnx` | L153 | `fn find_onnx(dir: &Path) -> Option<PathBuf>` |
| function | `pick_voice_id` | L166 | `fn pick_voice_id(voices: &[VoiceInfo], configured: &str) -> Option<String>` |
| function | `friendly_name` | L181 | `fn friendly_name(dir: &str) -> String` |
| function | `sanitize` | L202 | `fn sanitize(text: &str) -> String` |
| function | `chunk_text` | L208 | `fn chunk_text(text: &str) -> Vec<String>` |
| function | `push_trimmed` | L229 | `fn push_trimmed(out: &mut Vec<String>, s: String) -> ()` |
| function | `split_sentences` | L236 | `fn split_sentences(text: &str) -> Vec<String>` |
| function | `hard_split` | L251 | `fn hard_split(s: &str, max: usize) -> Vec<String>` |
| function | `tts_root_resolves_cross_platform_config_dir` | L275 | `fn tts_root_resolves_cross_platform_config_dir() -> ()` |
| function | `voice_dir_validation_rejects_traversal` | L282 | `fn voice_dir_validation_rejects_traversal() -> ()` |
