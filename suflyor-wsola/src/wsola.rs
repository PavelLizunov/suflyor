//! Derived from `timestretch` 0.5.0's MIT-licensed WSOLA implementation.

use crate::error::WsolaError;
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::Arc;

const COMPLEX_ZERO: Complex<f32> = Complex::new(0.0, 0.0);

/// Minimum energy threshold to avoid division by near-zero in correlation normalization.
const ENERGY_EPSILON: f64 = 1e-12;
/// Minimum number of candidates to justify FFT-based correlation over direct computation.
const FFT_CANDIDATE_THRESHOLD: usize = 64;
/// Minimum overlap length for FFT-based correlation to be worthwhile.
const FFT_OVERLAP_THRESHOLD: usize = 32;
/// Extra slack for loop-guard iteration bounds in dynamic WSOLA loops.
const LOOP_GUARD_SLACK: usize = 8;
/// Unroll factor for correlation kernels. This layout is friendly to
/// auto-vectorization on AVX2/NEON, with scalar cleanup for the tail.
const CORR_UNROLL: usize = 8;

/// WSOLA (Waveform Similarity Overlap-Add) time stretching.
///
/// Preserves transient quality better than phase vocoder by operating
/// in the time domain and finding optimal overlap positions via
/// cross-correlation.
pub struct Wsola {
    segment_size: usize,
    overlap_size: usize,
    search_range: usize,
    stretch_ratio: f64,
    planner: FftPlanner<f32>,
    /// Cached FFT size for correlation plan reuse.
    fft_plan_size: usize,
    /// Cached forward FFT plan for the current `fft_plan_size`.
    fft_fwd: Option<Arc<dyn rustfft::Fft<f32>>>,
    /// Cached inverse FFT plan for the current `fft_plan_size`.
    fft_inv: Option<Arc<dyn rustfft::Fft<f32>>>,
    /// Scratch for forward FFT execution.
    fft_fwd_scratch: Vec<Complex<f32>>,
    /// Scratch for inverse FFT execution.
    fft_inv_scratch: Vec<Complex<f32>>,
    /// Reusable FFT buffer for reference signal in cross-correlation.
    fft_ref_buf: Vec<Complex<f32>>,
    /// Reusable FFT buffer for search signal in cross-correlation.
    fft_search_buf: Vec<Complex<f32>>,
    /// Reusable FFT buffer for correlation result.
    fft_corr_buf: Vec<Complex<f32>>,
    /// Reusable prefix-sum buffer for energy normalization.
    prefix_sq_buf: Vec<f64>,
    /// Reusable output buffer for overlap-add accumulation.
    output_buf: Vec<f32>,
    /// Reusable correlation buffer for direct-search candidates.
    corr_values_buf: Vec<f64>,
    /// Reusable normalized-correlation buffer for FFT candidate scan.
    norm_corr_values_buf: Vec<f64>,
    /// Precomputed raised-cosine fade-in weights for overlap-add.
    crossfade_in: Vec<f32>,
    /// Precomputed raised-cosine fade-out weights for overlap-add.
    crossfade_out: Vec<f32>,
    /// When true, use equal-power (sin/cos) crossfade instead of
    /// constant-amplitude (raised cosine). Equal-power is correct for
    /// uncorrelated signals like percussive/noise content where
    /// constant-amplitude creates a ~3 dB energy dip at crossfade midpoints.
    equal_power: bool,
}

impl std::fmt::Debug for Wsola {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wsola")
            .field("segment_size", &self.segment_size)
            .field("overlap_size", &self.overlap_size)
            .field("search_range", &self.search_range)
            .field("stretch_ratio", &self.stretch_ratio)
            .finish()
    }
}

impl Wsola {
    /// Creates a new WSOLA processor.
    ///
    /// For small stretch ratios (within Ã‚Â±15% of unity), uses a smaller overlap
    /// region (`segment_size / 4`) to reduce transient smearing. Larger ratios
    /// use the standard `segment_size / 2` overlap for better continuity.
    pub fn new(segment_size: usize, search_range: usize, stretch_ratio: f64) -> Self {
        let overlap_size = overlap_for_ratio(segment_size, stretch_ratio);
        let max_overlap = segment_size / 2;
        let mut crossfade_in = vec![0.0; max_overlap];
        let mut crossfade_out = vec![0.0; max_overlap];
        fill_raised_cosine_crossfade(
            &mut crossfade_in[..overlap_size],
            &mut crossfade_out[..overlap_size],
        );
        Self {
            segment_size,
            overlap_size,
            search_range,
            stretch_ratio,
            planner: FftPlanner::new(),
            fft_plan_size: 0,
            fft_fwd: None,
            fft_inv: None,
            fft_fwd_scratch: Vec::new(),
            fft_inv_scratch: Vec::new(),
            fft_ref_buf: Vec::new(),
            fft_search_buf: Vec::new(),
            fft_corr_buf: Vec::new(),
            prefix_sq_buf: Vec::new(),
            output_buf: Vec::new(),
            corr_values_buf: Vec::new(),
            norm_corr_values_buf: Vec::new(),
            crossfade_in,
            crossfade_out,
            equal_power: false,
        }
    }

    /// Switches to equal-power crossfade (sin/cos curves).
    ///
    /// For uncorrelated signals (noise, percussive content), constant-amplitude
    /// crossfade (`fade_in + fade_out = 1`) creates a ~3 dB energy dip at the
    /// midpoint because the powers don't sum to unity. Equal-power crossfade
    /// (`fade_inÃ‚Â² + fade_outÃ‚Â² = 1`) maintains constant energy for uncorrelated
    /// signals, eliminating periodic spectral dips at overlap boundaries.
    pub fn set_equal_power_crossfade(&mut self) {
        self.equal_power = true;
        let n = self.overlap_size;
        if n == 0 {
            return;
        }
        let inv_n = 1.0 / n as f32;
        for i in 0..n {
            let t = i as f32 * inv_n;
            self.crossfade_in[i] = (std::f32::consts::FRAC_PI_2 * t).sin();
            self.crossfade_out[i] = (std::f32::consts::FRAC_PI_2 * t).cos();
        }
    }

    /// Returns the segment size in samples.
    #[inline]
    pub fn segment_size(&self) -> usize {
        self.segment_size
    }

    /// Returns the search range in samples.
    #[inline]
    pub fn search_range(&self) -> usize {
        self.search_range
    }

    /// Returns the stretch ratio.
    #[inline]
    pub fn stretch_ratio(&self) -> f64 {
        self.stretch_ratio
    }

    /// Updates the stretch ratio for subsequent processing.
    ///
    /// This also reconfigures overlap/crossfade geometry so near-unity ratios
    /// use a tighter overlap and farther-from-unity ratios use a longer overlap.
    pub fn set_stretch_ratio(&mut self, stretch_ratio: f64) {
        self.stretch_ratio = stretch_ratio;
        self.overlap_size = overlap_for_ratio(self.segment_size, stretch_ratio);
        self.rebuild_crossfade_tables();
        if self.equal_power {
            self.set_equal_power_crossfade();
        }
    }

    /// Reserves internal overlap-add storage for RT no-growth processing.
    ///
    /// This should be called during non-RT setup with worst-case `(input_len, ratio)`.
    pub fn reserve_output_capacity(&mut self, input_len: usize, max_ratio: f64) {
        let ratio = max_ratio.max(self.stretch_ratio).max(1.0);
        let target_output_len = (input_len as f64 * ratio).ceil() as usize;
        let needed = target_output_len.saturating_add(self.segment_size.saturating_mul(2));
        if self.output_buf.capacity() < needed {
            self.output_buf
                .reserve(needed.saturating_sub(self.output_buf.capacity()));
        }
        if self.output_buf.len() < needed {
            self.output_buf.resize(needed, 0.0);
        }
    }

    /// Stretches a mono audio signal using WSOLA.
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, WsolaError> {
        let mut out = Vec::new();
        self.process_into_internal(input, &mut out, true, true)?;
        Ok(out)
    }

    /// Stretches a mono signal into a caller-provided output buffer.
    ///
    /// This variant never grows `output`; if capacity is insufficient it returns
    /// `WsolaError::BufferOverflow`.
    pub fn process_into(&mut self, input: &[f32], output: &mut Vec<f32>) -> Result<(), WsolaError> {
        self.process_into_internal(input, output, false, true)
    }

    /// RT-focused variant that never grows internal buffers.
    ///
    /// Caller must pre-reserve internal capacity via
    /// [`Wsola::reserve_output_capacity`].
    pub fn process_into_no_grow(
        &mut self,
        input: &[f32],
        output: &mut Vec<f32>,
    ) -> Result<(), WsolaError> {
        self.process_into_internal(input, output, false, false)
    }

    fn process_into_internal(
        &mut self,
        input: &[f32],
        out: &mut Vec<f32>,
        allow_output_growth: bool,
        allow_internal_growth: bool,
    ) -> Result<(), WsolaError> {
        if self.segment_size == 0 {
            return Err(WsolaError::InvalidState("WSOLA segment_size must be > 0"));
        }
        if self.overlap_size >= self.segment_size {
            return Err(WsolaError::InvalidState(
                "WSOLA overlap_size must be < segment_size",
            ));
        }

        if input.len() < self.segment_size {
            return Err(WsolaError::InputTooShort {
                provided: input.len(),
                minimum: self.segment_size,
            });
        }

        let advance_input = self.segment_size - self.overlap_size;
        if advance_input == 0 {
            return Err(WsolaError::InvalidState(
                "WSOLA analysis advance must be > 0",
            ));
        }
        let advance_output_f = advance_input as f64 * self.stretch_ratio;

        if advance_output_f < 1.0 {
            return Err(WsolaError::InvalidRatio(
                "Stretch ratio too small for segment size".to_string(),
            ));
        }

        // Target output length based on stretch ratio
        let target_output_len = (input.len() as f64 * self.stretch_ratio).round() as usize;

        // Take the reusable buffer out of self to avoid borrow conflicts
        // (find_best_position borrows &mut self while output is also needed)
        let mut work = std::mem::take(&mut self.output_buf);

        // Grow if needed, zero the portion we'll use; never shrink
        let estimated_output_len = target_output_len + self.segment_size * 2;
        if work.capacity() < estimated_output_len {
            if allow_internal_growth {
                work.reserve(estimated_output_len.saturating_sub(work.capacity()));
            } else {
                self.output_buf = work;
                return Err(WsolaError::BufferOverflow {
                    buffer: "wsola_internal_output_buf",
                    requested: estimated_output_len,
                    available: self.output_buf.capacity(),
                });
            }
        }
        if work.len() < estimated_output_len {
            work.resize(estimated_output_len, 0.0);
        } else {
            for s in &mut work[..estimated_output_len] {
                *s = 0.0;
            }
        }

        // Copy first segment
        let first_len = self.segment_size.min(input.len());
        work[..first_len].copy_from_slice(&input[..first_len]);

        let mut input_pos: f64 = advance_input as f64;
        // Track output position fractionally to avoid cumulative rounding error
        let mut output_pos_f: f64 = advance_output_f;
        let mut actual_output_len = first_len;
        let mut iterations = 0usize;
        let max_iterations = input
            .len()
            .saturating_sub(self.segment_size)
            .saturating_div(advance_input)
            .saturating_add(LOOP_GUARD_SLACK);

        while (input_pos as usize) + self.segment_size <= input.len() {
            iterations = iterations.saturating_add(1);
            if iterations > max_iterations {
                self.output_buf = work;
                return Err(WsolaError::InvalidState(
                    "WSOLA main loop iteration bound exceeded",
                ));
            }
            // For compression (ratio < 1.0), stop once we've produced enough output
            if actual_output_len >= target_output_len {
                break;
            }

            let nominal_pos = input_pos as usize;
            let output_pos = output_pos_f.round() as usize;

            // Ensure we have room in the output buffer
            let needed = output_pos + self.segment_size;
            if needed > work.capacity() {
                if allow_internal_growth {
                    work.reserve(needed.saturating_sub(work.capacity()));
                } else {
                    self.output_buf = work;
                    return Err(WsolaError::BufferOverflow {
                        buffer: "wsola_internal_output_buf",
                        requested: needed,
                        available: self.output_buf.capacity(),
                    });
                }
            }
            if needed > work.len() {
                work.resize(needed, 0.0);
            }

            // Search for best matching position around nominal position
            let (best_pos, fractional_offset) =
                self.find_best_position(input, &work, nominal_pos, output_pos);

            // Overlap-add with cross-fade (using sub-sample offset for precision)
            self.overlap_add(input, &mut work, best_pos, output_pos, fractional_offset);
            actual_output_len = (output_pos + self.segment_size).max(actual_output_len);

            input_pos += advance_input as f64;
            output_pos_f += advance_output_f;
        }

        let final_len = actual_output_len.min(target_output_len);
        if !allow_output_growth && out.capacity() < final_len {
            self.output_buf = work;
            return Err(WsolaError::BufferOverflow {
                buffer: "wsola_process_into_output",
                requested: final_len,
                available: out.capacity(),
            });
        }
        out.clear();
        out.extend_from_slice(&work[..final_len]);
        self.output_buf = work;
        Ok(())
    }

    /// Finds the best matching position within the search range using FFT-accelerated
    /// cross-correlation for large search ranges, falling back to direct computation
    /// for small ranges.
    ///
    /// Returns `(integer_position, fractional_offset)` where the true best alignment
    /// is at `integer_position + fractional_offset` samples. The fractional offset
    /// is determined via parabolic interpolation of the correlation peak.
    fn find_best_position(
        &mut self,
        input: &[f32],
        output: &[f32],
        nominal_pos: usize,
        output_pos: usize,
    ) -> (usize, f64) {
        let search_start = nominal_pos.saturating_sub(self.search_range);
        let search_end =
            (nominal_pos + self.search_range).min(input.len().saturating_sub(self.segment_size));

        if search_start >= search_end {
            return (
                nominal_pos.min(input.len().saturating_sub(self.segment_size)),
                0.0,
            );
        }

        let overlap_len = self
            .overlap_size
            .min(output.len().saturating_sub(output_pos));
        if overlap_len == 0 {
            return (nominal_pos, 0.0);
        }

        let num_candidates = search_end - search_start + 1;

        // Use FFT-based correlation when search range is large enough to benefit
        if num_candidates > FFT_CANDIDATE_THRESHOLD && overlap_len >= FFT_OVERLAP_THRESHOLD {
            self.find_best_position_fft(
                input,
                output,
                search_start,
                search_end,
                output_pos,
                overlap_len,
            )
        } else {
            self.find_best_position_direct(
                input,
                output,
                search_start,
                search_end,
                output_pos,
                overlap_len,
            )
        }
    }

    /// Direct time-domain cross-correlation search (used for small search ranges).
    ///
    /// Returns `(integer_position, fractional_offset)` with parabolic refinement
    /// of the correlation peak for sub-sample accuracy.
    fn find_best_position_direct(
        &mut self,
        input: &[f32],
        output: &[f32],
        search_start: usize,
        search_end: usize,
        output_pos: usize,
        overlap_len: usize,
    ) -> (usize, f64) {
        let mut best_pos = search_start;
        let mut best_corr = f64::NEG_INFINITY;
        let ref_slice = &output[output_pos..output_pos + overlap_len];
        let (ref_sum, ref_sum2) = sum_and_square_sum(ref_slice);
        let n = ref_slice.len() as f64;
        let ref_var = ref_sum2 - (ref_sum * ref_sum) / n.max(1.0);
        if ref_var <= ENERGY_EPSILON {
            return (search_start, 0.0);
        }

        // Collect correlation values for parabolic interpolation
        let num_candidates = search_end - search_start + 1;
        self.corr_values_buf.resize(num_candidates, 0.0);
        let mut computed = 0usize;

        for (idx, pos) in (search_start..=search_end).enumerate() {
            if pos + overlap_len > input.len() {
                break;
            }

            let corr = normalized_cross_correlation_with_reference_stats(
                ref_slice,
                ref_sum,
                ref_sum2,
                ref_var,
                &input[pos..pos + overlap_len],
            );
            self.corr_values_buf[idx] = corr;
            computed = idx + 1;

            if corr > best_corr {
                best_corr = corr;
                best_pos = pos;
            }
        }
        self.corr_values_buf.truncate(computed);

        // Parabolic interpolation for sub-sample accuracy
        let best_idx = best_pos - search_start;
        let fractional_offset = parabolic_interpolation(&self.corr_values_buf, best_idx);

        (best_pos, fractional_offset)
    }

    /// FFT-accelerated cross-correlation search.
    ///
    /// Computes cross-correlation between the output overlap region (reference)
    /// and all candidate positions in the input search region simultaneously.
    ///
    /// Returns `(integer_position, fractional_offset)` with parabolic refinement
    /// of the correlation peak for sub-sample accuracy.
    fn find_best_position_fft(
        &mut self,
        input: &[f32],
        output: &[f32],
        search_start: usize,
        search_end: usize,
        output_pos: usize,
        overlap_len: usize,
    ) -> (usize, f64) {
        let ref_signal = &output[output_pos..output_pos + overlap_len];
        let search_region_len = search_end - search_start + overlap_len;

        // Clamp to available input
        let actual_region_end = (search_start + search_region_len).min(input.len());
        let actual_region_len = actual_region_end - search_start;
        if actual_region_len < overlap_len {
            return (search_start, 0.0);
        }
        let search_signal = &input[search_start..actual_region_end];

        // Compute raw cross-correlation via FFT (results stored in self.fft_corr_buf)
        self.fft_cross_correlate(ref_signal, search_signal);

        // Compute reference energy (constant for all candidates)
        let ref_energy: f64 = ref_signal.iter().map(|&s| (s as f64) * (s as f64)).sum();
        if ref_energy < ENERGY_EPSILON {
            return (search_start, 0.0);
        }

        // Find best candidate using normalized correlation
        let num_candidates = actual_region_len.saturating_sub(overlap_len) + 1;

        // Reuse prefix_sq_buf for energy normalization
        self.prefix_sq_buf.resize(search_signal.len() + 1, 0.0);
        let mut accum = 0.0f64;
        for (i, &s) in search_signal.iter().enumerate() {
            accum += (s as f64) * (s as f64);
            self.prefix_sq_buf[i + 1] = accum;
        }

        let (best_pos, fractional_offset) = find_best_candidate(
            &self.prefix_sq_buf,
            &self.fft_corr_buf,
            ref_energy,
            num_candidates,
            overlap_len,
            search_start,
            &mut self.norm_corr_values_buf,
        );

        // Clamp to valid range
        (best_pos.min(search_end), fractional_offset)
    }

    /// Computes cross-correlation between two signals using FFT.
    ///
    /// Uses pre-allocated buffers that grow as needed but never shrink,
    /// eliminating per-call heap allocations in the hot path.
    fn fft_cross_correlate(&mut self, ref_signal: &[f32], search_signal: &[f32]) {
        let conv_len = search_signal.len() + ref_signal.len() - 1;
        let fft_size = conv_len.next_power_of_two();

        self.ensure_fft_plan(fft_size);
        let (Some(fft_fwd), Some(fft_inv)) = (self.fft_fwd.as_ref(), self.fft_inv.as_ref()) else {
            return;
        };
        let fft_fwd = fft_fwd.clone();
        let fft_inv = fft_inv.clone();

        // Resize and fill reusable buffers (grow-only, never shrink).
        // Zero-fill first, then copy signal data Ã¢â‚¬â€ avoids per-element branch
        // which inhibits auto-vectorization.
        self.fft_ref_buf.resize(fft_size, COMPLEX_ZERO);
        self.fft_ref_buf.fill(COMPLEX_ZERO);
        for (slot, &s) in self.fft_ref_buf.iter_mut().zip(ref_signal.iter()) {
            *slot = Complex::new(s, 0.0);
        }

        self.fft_search_buf.resize(fft_size, COMPLEX_ZERO);
        self.fft_search_buf.fill(COMPLEX_ZERO);
        for (slot, &s) in self.fft_search_buf.iter_mut().zip(search_signal.iter()) {
            *slot = Complex::new(s, 0.0);
        }

        // Forward FFT
        fft_fwd.process_with_scratch(&mut self.fft_ref_buf, &mut self.fft_fwd_scratch);
        fft_fwd.process_with_scratch(&mut self.fft_search_buf, &mut self.fft_fwd_scratch);

        // Multiply conj(Ref) * Search into corr_buf (index-based for auto-vectorization)
        self.fft_corr_buf.resize(fft_size, COMPLEX_ZERO);
        for i in 0..fft_size {
            self.fft_corr_buf[i] = self.fft_ref_buf[i].conj() * self.fft_search_buf[i];
        }

        // Inverse FFT in-place
        fft_inv.process_with_scratch(&mut self.fft_corr_buf, &mut self.fft_inv_scratch);
    }

    /// Ensures cached FFT plans/scratch match `fft_size`.
    fn ensure_fft_plan(&mut self, fft_size: usize) {
        if self.fft_plan_size == fft_size && self.fft_fwd.is_some() && self.fft_inv.is_some() {
            return;
        }

        let fft_fwd = self.planner.plan_fft_forward(fft_size);
        let fft_inv = self.planner.plan_fft_inverse(fft_size);
        let fwd_scratch = fft_fwd.get_inplace_scratch_len();
        let inv_scratch = fft_inv.get_inplace_scratch_len();

        self.fft_plan_size = fft_size;
        self.fft_fwd = Some(fft_fwd);
        self.fft_inv = Some(fft_inv);
        self.fft_fwd_scratch.resize(fwd_scratch, COMPLEX_ZERO);
        self.fft_inv_scratch.resize(inv_scratch, COMPLEX_ZERO);
    }

    /// Overlap-adds a segment from input into output with raised-cosine crossfade.
    ///
    /// When `fractional_offset` is non-zero, applies sub-sample interpolation to the
    /// input read positions for pitch-drift-free alignment. The fractional offset
    /// shifts the source read by a sub-sample amount using linear interpolation.
    ///
    /// Split into two separate loops (crossfade region vs copy region) so each
    /// loop body is branch-free and amenable to auto-vectorization.
    #[inline]
    fn overlap_add(
        &self,
        input: &[f32],
        output: &mut [f32],
        input_pos: usize,
        output_pos: usize,
        fractional_offset: f64,
    ) {
        let segment_end = (input_pos + self.segment_size).min(input.len());
        let segment_len = segment_end - input_pos;
        let out_avail = output.len().saturating_sub(output_pos);
        let len = segment_len.min(out_avail);

        // If we have a fractional offset that would require reading past the end,
        // reduce len by 1 to leave room for the interpolation neighbor.
        let len = if fractional_offset.abs() > 1e-10 && len > 0 {
            // Need src_idx + 1 < input.len() for the last sample
            let last_src = input_pos as f64 + (len - 1) as f64 + fractional_offset;
            let last_idx = last_src.floor() as usize;
            if last_idx + 1 >= input.len() {
                len.saturating_sub(1)
            } else {
                len
            }
        } else {
            len
        };

        let overlap_len = self.overlap_size.min(len);
        let use_interp = fractional_offset.abs() > 1e-10;

        // For expansion (ratio > 1.0), the output advance exceeds the segment's
        // non-overlap region, so the tail of the crossfade zone overlaps with
        // zero-filled output (the previous segment didn't reach this far).
        // Crossfading new content with zeros would create an amplitude dip.
        // Only crossfade where real previous content exists; write full-amplitude
        // new content in the gap region.
        let valid_overlap = if self.stretch_ratio > 1.0 {
            let advance_input = self.segment_size - self.overlap_size;
            let advance_output = (advance_input as f64 * self.stretch_ratio).round() as usize;
            if advance_output < self.segment_size {
                (self.segment_size - advance_output).min(overlap_len)
            } else {
                overlap_len
            }
        } else {
            overlap_len
        };

        // Crossfade region: raised-cosine fade where previous content exists.
        // For expansion, valid_overlap < overlap_size, so we rescale the
        // crossfade to span the full 0Ã¢â€ â€™1 range within valid_overlap samples.
        // Without rescaling, the crossfade only reaches ~50% at the gap
        // boundary, creating a hard amplitude jump that produces comb-filtering
        // artifacts on broadband (noise-like) percussive content.
        let need_rescale = valid_overlap > 0 && valid_overlap < overlap_len;
        let inv_valid = 1.0 / valid_overlap.max(1) as f32;
        for i in 0..valid_overlap {
            let (fade_in, fade_out) = if need_rescale {
                let t = i as f32 * inv_valid;
                if self.equal_power {
                    let fi = (std::f32::consts::FRAC_PI_2 * t).sin();
                    (fi, (std::f32::consts::FRAC_PI_2 * t).cos())
                } else {
                    let fi = 0.5 * (1.0 - (std::f32::consts::PI * t).cos());
                    (fi, 1.0 - fi)
                }
            } else {
                (self.crossfade_in[i], self.crossfade_out[i])
            };
            let in_sample = if use_interp {
                subsample_interpolate(input, input_pos, i, fractional_offset)
            } else {
                input[input_pos + i]
            };
            output[output_pos + i] = output[output_pos + i] * fade_out + in_sample * fade_in;
        }

        // Gap region: previous segment didn't reach here (output is zero).
        // Write new content at full amplitude to avoid the dip artifact.
        if use_interp {
            for i in valid_overlap..overlap_len {
                output[output_pos + i] =
                    subsample_interpolate(input, input_pos, i, fractional_offset);
            }
        } else if valid_overlap < overlap_len {
            output[output_pos + valid_overlap..output_pos + overlap_len]
                .copy_from_slice(&input[input_pos + valid_overlap..input_pos + overlap_len]);
        }

        // Non-overlap region
        if use_interp {
            // Sub-sample interpolated copy
            for i in overlap_len..len {
                output[output_pos + i] =
                    subsample_interpolate(input, input_pos, i, fractional_offset);
            }
        } else {
            // Direct copy (fast path, no fractional offset)
            let copy_start = overlap_len;
            output[output_pos + copy_start..output_pos + len]
                .copy_from_slice(&input[input_pos + copy_start..input_pos + len]);
        }
    }

    #[inline]
    fn rebuild_crossfade_tables(&mut self) {
        if self.overlap_size == 0 {
            return;
        }
        debug_assert!(self.overlap_size <= self.crossfade_in.len());
        debug_assert!(self.overlap_size <= self.crossfade_out.len());
        fill_raised_cosine_crossfade(
            &mut self.crossfade_in[..self.overlap_size],
            &mut self.crossfade_out[..self.overlap_size],
        );
    }
}

#[inline]
fn overlap_for_ratio(segment_size: usize, stretch_ratio: f64) -> usize {
    if (stretch_ratio - 1.0).abs() < 0.15 {
        segment_size / 4
    } else {
        segment_size / 2
    }
}

fn fill_raised_cosine_crossfade(fade_in: &mut [f32], fade_out: &mut [f32]) {
    debug_assert_eq!(fade_in.len(), fade_out.len());
    let overlap_size = fade_in.len();
    if overlap_size == 0 {
        return;
    }

    let inv_overlap = 1.0 / overlap_size as f32;
    for i in 0..overlap_size {
        let t = i as f32 * inv_overlap;
        let fi = 0.5 * (1.0 - (std::f32::consts::PI * t).cos());
        fade_in[i] = fi;
        fade_out[i] = 1.0 - fi;
    }
}

/// Finds the best correlation candidate using prefix-sum energy normalization.
///
/// Scans `num_candidates` lag positions in `corr_buf`, normalizing each by
/// the windowed energy (via pre-computed `prefix_sq`) and the reference energy.
///
/// Returns `(integer_position, fractional_offset)` with parabolic refinement
/// of the correlation peak for sub-sample accuracy.
fn find_best_candidate(
    prefix_sq: &[f64],
    corr_buf: &[Complex<f32>],
    ref_energy: f64,
    num_candidates: usize,
    overlap_len: usize,
    search_start: usize,
    norm_corr_values: &mut Vec<f64>,
) -> (usize, f64) {
    let norm = 1.0 / corr_buf.len() as f64;

    let mut best_pos = search_start;
    let mut best_ncorr = f64::NEG_INFINITY;
    let mut best_k: usize = 0;

    // Collect normalized correlation values for parabolic interpolation
    norm_corr_values.resize(num_candidates, 0.0);

    for k in 0..num_candidates {
        let raw_corr = corr_buf[k].re as f64 * norm;
        let window_energy = prefix_sq[k + overlap_len] - prefix_sq[k];
        let denom = (ref_energy * window_energy).sqrt();

        let ncorr = if denom > ENERGY_EPSILON {
            raw_corr / denom
        } else {
            0.0
        };

        norm_corr_values[k] = ncorr;

        if ncorr > best_ncorr {
            best_ncorr = ncorr;
            best_pos = search_start + k;
            best_k = k;
        }
    }

    // Parabolic interpolation for sub-sample accuracy
    let fractional_offset = parabolic_interpolation(norm_corr_values, best_k);

    (best_pos, fractional_offset)
}

/// Computes `sum(x)` and `sum(x^2)` in one pass.
///
/// The unrolled structure is intentionally simple so LLVM can map it to
/// platform SIMD where available (AVX2/NEON) and scalar fallback otherwise.
#[inline]
fn sum_and_square_sum(x: &[f32]) -> (f64, f64) {
    let n = x.len();
    let mut sum0 = 0.0f64;
    let mut sum1 = 0.0f64;
    let mut sum2 = 0.0f64;
    let mut sum3 = 0.0f64;
    let mut sum4 = 0.0f64;
    let mut sum5 = 0.0f64;
    let mut sum6 = 0.0f64;
    let mut sum7 = 0.0f64;
    let mut sq0 = 0.0f64;
    let mut sq1 = 0.0f64;
    let mut sq2 = 0.0f64;
    let mut sq3 = 0.0f64;
    let mut sq4 = 0.0f64;
    let mut sq5 = 0.0f64;
    let mut sq6 = 0.0f64;
    let mut sq7 = 0.0f64;

    let mut i = 0usize;
    while i + CORR_UNROLL <= n {
        let v0 = x[i] as f64;
        let v1 = x[i + 1] as f64;
        let v2 = x[i + 2] as f64;
        let v3 = x[i + 3] as f64;
        let v4 = x[i + 4] as f64;
        let v5 = x[i + 5] as f64;
        let v6 = x[i + 6] as f64;
        let v7 = x[i + 7] as f64;

        sum0 += v0;
        sum1 += v1;
        sum2 += v2;
        sum3 += v3;
        sum4 += v4;
        sum5 += v5;
        sum6 += v6;
        sum7 += v7;
        sq0 += v0 * v0;
        sq1 += v1 * v1;
        sq2 += v2 * v2;
        sq3 += v3 * v3;
        sq4 += v4 * v4;
        sq5 += v5 * v5;
        sq6 += v6 * v6;
        sq7 += v7 * v7;
        i += CORR_UNROLL;
    }

    let mut sum = sum0 + sum1 + sum2 + sum3 + sum4 + sum5 + sum6 + sum7;
    let mut sum_sq = sq0 + sq1 + sq2 + sq3 + sq4 + sq5 + sq6 + sq7;
    while i < n {
        let v = x[i] as f64;
        sum += v;
        sum_sq += v * v;
        i += 1;
    }
    (sum, sum_sq)
}

/// Computes `sum(y)` and `sum(y^2)` and `sum(x*y)` in one pass.
///
/// Uses the same unrolled SIMD-friendly structure as [`sum_and_square_sum`].
#[inline]
fn sum_cross_terms(x: &[f32], y: &[f32]) -> (f64, f64, f64) {
    let n = x.len().min(y.len());
    let mut ysum0 = 0.0f64;
    let mut ysum1 = 0.0f64;
    let mut ysum2 = 0.0f64;
    let mut ysum3 = 0.0f64;
    let mut ysum4 = 0.0f64;
    let mut ysum5 = 0.0f64;
    let mut ysum6 = 0.0f64;
    let mut ysum7 = 0.0f64;
    let mut ysq0 = 0.0f64;
    let mut ysq1 = 0.0f64;
    let mut ysq2 = 0.0f64;
    let mut ysq3 = 0.0f64;
    let mut ysq4 = 0.0f64;
    let mut ysq5 = 0.0f64;
    let mut ysq6 = 0.0f64;
    let mut ysq7 = 0.0f64;
    let mut xy0 = 0.0f64;
    let mut xy1 = 0.0f64;
    let mut xy2 = 0.0f64;
    let mut xy3 = 0.0f64;
    let mut xy4 = 0.0f64;
    let mut xy5 = 0.0f64;
    let mut xy6 = 0.0f64;
    let mut xy7 = 0.0f64;

    let mut i = 0usize;
    while i + CORR_UNROLL <= n {
        let x0 = x[i] as f64;
        let x1 = x[i + 1] as f64;
        let x2 = x[i + 2] as f64;
        let x3 = x[i + 3] as f64;
        let x4 = x[i + 4] as f64;
        let x5 = x[i + 5] as f64;
        let x6 = x[i + 6] as f64;
        let x7 = x[i + 7] as f64;
        let y0 = y[i] as f64;
        let y1 = y[i + 1] as f64;
        let y2 = y[i + 2] as f64;
        let y3 = y[i + 3] as f64;
        let y4 = y[i + 4] as f64;
        let y5 = y[i + 5] as f64;
        let y6 = y[i + 6] as f64;
        let y7 = y[i + 7] as f64;

        ysum0 += y0;
        ysum1 += y1;
        ysum2 += y2;
        ysum3 += y3;
        ysum4 += y4;
        ysum5 += y5;
        ysum6 += y6;
        ysum7 += y7;
        ysq0 += y0 * y0;
        ysq1 += y1 * y1;
        ysq2 += y2 * y2;
        ysq3 += y3 * y3;
        ysq4 += y4 * y4;
        ysq5 += y5 * y5;
        ysq6 += y6 * y6;
        ysq7 += y7 * y7;
        xy0 += x0 * y0;
        xy1 += x1 * y1;
        xy2 += x2 * y2;
        xy3 += x3 * y3;
        xy4 += x4 * y4;
        xy5 += x5 * y5;
        xy6 += x6 * y6;
        xy7 += x7 * y7;
        i += CORR_UNROLL;
    }

    let mut sum_y = ysum0 + ysum1 + ysum2 + ysum3 + ysum4 + ysum5 + ysum6 + ysum7;
    let mut sum_y2 = ysq0 + ysq1 + ysq2 + ysq3 + ysq4 + ysq5 + ysq6 + ysq7;
    let mut sum_xy = xy0 + xy1 + xy2 + xy3 + xy4 + xy5 + xy6 + xy7;
    while i < n {
        let xv = x[i] as f64;
        let yv = y[i] as f64;
        sum_y += yv;
        sum_y2 += yv * yv;
        sum_xy += xv * yv;
        i += 1;
    }
    (sum_y, sum_y2, sum_xy)
}

#[inline]
fn normalized_cross_correlation_with_reference_stats(
    reference: &[f32],
    ref_sum: f64,
    ref_sum2: f64,
    ref_var: f64,
    candidate: &[f32],
) -> f64 {
    let n = reference.len().min(candidate.len());
    if n == 0 {
        return 0.0;
    }

    let n_f = n as f64;
    let reference = &reference[..n];
    let candidate = &candidate[..n];
    let (sum_b, sum_b2, sum_ab) = sum_cross_terms(reference, candidate);
    let numerator = sum_ab - (ref_sum * sum_b / n_f);
    let var_b = sum_b2 - (sum_b * sum_b / n_f);
    if var_b <= ENERGY_EPSILON || ref_var <= ENERGY_EPSILON {
        return 0.0;
    }

    // Keep the explicit use of ref_sum2 to avoid recalculation in callers.
    let _ = ref_sum2;
    numerator / (ref_var * var_b).sqrt()
}

/// Parabolic interpolation for sub-sample peak refinement.
///
/// Given a vector of correlation values and the index `k` of the integer peak,
/// fits a parabola through `corr[k-1]`, `corr[k]`, `corr[k+1]` and returns the
/// fractional offset `p` in `[-0.5, 0.5]` of the true peak relative to `k`.
#[inline]
fn parabolic_interpolation(corr: &[f64], k: usize) -> f64 {
    if k == 0 || k >= corr.len() - 1 || corr.len() < 3 {
        return 0.0;
    }

    let alpha = corr[k - 1];
    let beta = corr[k];
    let gamma = corr[k + 1];
    let denom = alpha - 2.0 * beta + gamma;

    if denom.abs() > 1e-10 {
        let p = 0.5 * (alpha - gamma) / denom;
        // Clamp to [-0.5, 0.5] for safety
        p.clamp(-0.5, 0.5)
    } else {
        0.0
    }
}

/// Reads a sample from `input` at sub-sample position `input_pos + i + fractional_offset`
/// using linear interpolation between adjacent samples.
#[inline]
fn subsample_interpolate(input: &[f32], input_pos: usize, i: usize, fractional_offset: f64) -> f32 {
    let src_pos = (input_pos + i) as f64 + fractional_offset;
    let src_idx = src_pos.floor() as usize;
    let frac = (src_pos - src_pos.floor()) as f32;

    if src_idx + 1 < input.len() {
        input[src_idx] * (1.0 - frac) + input[src_idx + 1] * frac
    } else if src_idx < input.len() {
        input[src_idx]
    } else {
        0.0
    }
}
