---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ba1e685b595a"
source_path: "suflyor-wsola/src/wsola.rs"
batch_id: "B13"
total_lines: 1018
symbols_count: 27
review_state: validated
---

# File Map: `suflyor-wsola/src/wsola.rs`

- **Batch:** B13
- **Physical Lines:** 1018
- **Coverage:** 1018/1018 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `Wsola` | L26 | pub |

## Symbols & Routines (27)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `fmt` | L68 | `fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` |
| function | `new` | L84 | `fn new(segment_size: usize, search_range: usize, stretch_ratio: f64) -> Self` |
| function | `set_equal_power_crossfade` | L124 | `fn set_equal_power_crossfade(&mut self) -> ()` |
| function | `segment_size` | L140 | `fn segment_size(&self) -> usize` |
| function | `search_range` | L146 | `fn search_range(&self) -> usize` |
| function | `stretch_ratio` | L152 | `fn stretch_ratio(&self) -> f64` |
| function | `set_stretch_ratio` | L160 | `fn set_stretch_ratio(&mut self, stretch_ratio: f64) -> ()` |
| function | `reserve_output_capacity` | L172 | `fn reserve_output_capacity(&mut self, input_len: usize, max_ratio: f64) -> ()` |
| function | `process` | L186 | `fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, WsolaError>` |
| function | `process_into` | L196 | `fn process_into(&mut self, input: &[f32], output: &mut Vec<f32>) -> Result<(), WsolaError>` |
| function | `process_into_no_grow` | L204 | `fn process_into_no_grow(&mut self, input: &[f32], output: &mut Vec<f32>,) -> Result<(), WsolaError>` |
| function | `process_into_internal` | L212 | `fn process_into_internal(&mut self, input: &[f32], out: &mut Vec<f32>, allow_output_growth: bool, allow_internal_growth: bool,) -> Result<(), WsolaError>` |
| function | `find_best_position` | L361 | `fn find_best_position(&mut self, input: &[f32], output: &[f32], nominal_pos: usize, output_pos: usize,) -> (usize, f64)` |
| function | `find_best_position_direct` | L414 | `fn find_best_position_direct(&mut self, input: &[f32], output: &[f32], search_start: usize, search_end: usize, output_pos: usize, overlap_len: usize,) -> (usize, f64)` |
| function | `find_best_position_fft` | L474 | `fn find_best_position_fft(&mut self, input: &[f32], output: &[f32], search_start: usize, search_end: usize, output_pos: usize, overlap_len: usize,) -> (usize, f64)` |
| function | `fft_cross_correlate` | L532 | `fn fft_cross_correlate(&mut self, ref_signal: &[f32], search_signal: &[f32]) -> ()` |
| function | `ensure_fft_plan` | L573 | `fn ensure_fft_plan(&mut self, fft_size: usize) -> ()` |
| function | `overlap_add` | L599 | `fn overlap_add(&self, input: &[f32], output: &mut [f32], input_pos: usize, output_pos: usize, fractional_offset: f64,) -> ()` |
| function | `rebuild_crossfade_tables` | L705 | `fn rebuild_crossfade_tables(&mut self) -> ()` |
| function | `overlap_for_ratio` | L719 | `fn overlap_for_ratio(segment_size: usize, stretch_ratio: f64) -> usize` |
| function | `fill_raised_cosine_crossfade` | L727 | `fn fill_raised_cosine_crossfade(fade_in: &mut [f32], fade_out: &mut [f32]) -> ()` |
| function | `find_best_candidate` | L750 | `fn find_best_candidate(prefix_sq: &[f64], corr_buf: &[Complex<f32>], ref_energy: f64, num_candidates: usize, overlap_len: usize, search_start: usize, norm_corr_values: &mut Vec<f64>,) -> (usize, f64)` |
| function | `sum_and_square_sum` | L799 | `fn sum_and_square_sum(x: &[f32]) -> (f64, f64)` |
| function | `sum_cross_terms` | L863 | `fn sum_cross_terms(x: &[f32], y: &[f32]) -> (f64, f64, f64)` |
| function | `normalized_cross_correlation_with_reference_stats` | L951 | `fn normalized_cross_correlation_with_reference_stats(reference: &[f32], ref_sum: f64, ref_sum2: f64, ref_var: f64, candidate: &[f32],) -> f64` |
| function | `parabolic_interpolation` | L984 | `fn parabolic_interpolation(corr: &[f64], k: usize) -> f64` |
| function | `subsample_interpolate` | L1006 | `fn subsample_interpolate(input: &[f32], input_pos: usize, i: usize, fractional_offset: f64) -> f32` |
