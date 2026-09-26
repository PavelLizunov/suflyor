---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_fd1c17174da1"
source_path: "suflyor-teratts/src/tera.rs"
batch_id: "B12"
total_lines: 546
symbols_count: 19
review_state: validated
---

# File Map: `suflyor-teratts/src/tera.rs`

- **Batch:** B12
- **Physical Lines:** 546
- **Coverage:** 546/546 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `TeraEngine` | L51 | pub |
| struct | `SynthOutput` | L67 | pub |

## Symbols & Routines (19)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `load` | L74 | `fn load(tts_root: &Path) -> Result<TeraEngine>` |
| function | `synthesize` | L132 | `fn synthesize(&mut self, text: &str, voice: &str, lang: &str, duration_scale: f32, seed: u64,) -> Result<SynthOutput>` |
| function | `load_style` | L308 | `fn load_style(&self, voice: &str, file: &str, shape: &[usize]) -> Result<NpyArray>` |
| function | `slice_latent_frames` | L324 | `fn slice_latent_frames(latent: &[f32], channels: usize, total_frames: usize, start: usize, end: usize,) -> Result<Vec<f32>>` |
| function | `load_session` | L349 | `fn load_session(path: &Path) -> Result<Session>` |
| function | `sole_declared_output` | L361 | `fn sole_declared_output(graph: &str, names: impl IntoIterator<Item = &'a str>,) -> Result<String>` |
| function | `named_output_f32` | L378 | `fn named_output_f32(outputs: &ort::session::SessionOutputs<'_>, name: &str,) -> Result<(Vec<usize>, Vec<f32>)>` |
| function | `validate_tensor_shape` | L401 | `fn validate_tensor_shape(shape: &[usize], data_len: usize, what: &str) -> Result<()>` |
| function | `shape_product` | L410 | `fn shape_product(shape: &[usize]) -> Result<usize>` |
| function | `validate_latent_output` | L429 | `fn validate_latent_output(shape: &[usize], data_len: usize, latent_length: usize) -> Result<()>` |
| function | `validate_vocoder_output` | L439 | `fn validate_vocoder_output(shape: &[usize], data_len: usize, min_samples: usize) -> Result<()>` |
| function | `constants_match_the_reference_release` | L459 | `fn constants_match_the_reference_release() -> ()` |
| function | `load_fails_with_not_installed_when_dir_absent` | L470 | `fn load_fails_with_not_installed_when_dir_absent() -> ()` |
| function | `sole_declared_output_rejects_ambiguity` | L479 | `fn sole_declared_output_rejects_ambiguity() -> ()` |
| function | `shape_product_rejects_empty_zero_and_overflow` | L491 | `fn shape_product_rejects_empty_zero_and_overflow() -> ()` |
| function | `tensor_shape_validation_requires_exact_lengths` | L499 | `fn tensor_shape_validation_requires_exact_lengths() -> ()` |
| function | `latent_output_validation_rejects_malformed_shapes` | L508 | `fn latent_output_validation_rejects_malformed_shapes() -> ()` |
| function | `vocoder_output_validation_rejects_short_or_misshapen_waveforms` | L522 | `fn vocoder_output_validation_rejects_short_or_misshapen_waveforms() -> ()` |
| function | `latent_frame_window_preserves_channel_first_layout` | L532 | `fn latent_frame_window_preserves_channel_first_layout() -> ()` |
