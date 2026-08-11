//! ONNX Runtime orchestration of the published TeraTTSv2 graphs.
//!
//! Exact contract (pinned revision, see `manifest/teratts-v2.json`), mirrored
//! from the upstream reference `teratts.py`:
//!
//!   text_encoder(text_ids i64[1,N], style_ttl f32[1,50,256],
//!                text_mask f32[1,1,N])            -> text_emb
//!   duration_predictor(text_ids, style_dp f32[1,8,16], text_mask) -> duration
//!   latent frames  = ceil(seconds * 44100 / 3072)
//!   sampler(initial_latent f32[1,144,L], text_emb, style_ttl,
//!           latent_mask f32[1,1,L], text_mask, guidance f32[1]) -> latent
//!   vocoder(latent f32[1,144,F]) -> waveform f32[1, F*3072]
//!
//! The distilled 8-step sampler owns its diffusion schedule; guidance stays at
//! the reference default 3.0. Vocoder decoding uses the reference causal
//! overlap-save streaming (20-frame context, 16-frame chunks).
//!
//! Output schema: every pinned graph declares EXACTLY ONE output tensor; the
//! engine records each declared name at load time and selects runtime outputs
//! by that exact name. Graphs with zero or multiple declared outputs are
//! rejected at load — output choice is never positional iteration order.
//! Every output shape/data length is validated BEFORE any slice or index, so
//! a mismatched model can only produce a generic `synth` protocol failure,
//! never a panic and never user text.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{anyhow, Result};
use ort::session::Session;
use ort::value::Tensor;

use crate::indexer::UnicodeIndexer;
use crate::manifest::{self, Manifest};
use crate::npy::{self, NpyArray};
use crate::rng::Rng;
use crate::textnorm;

pub const SAMPLE_RATE: u32 = 44_100;
pub const SAMPLES_PER_COMPRESSED_FRAME: usize = 3_072;
pub const VOCODER_CONTEXT_FRAMES: usize = 20;
pub const STREAM_CHUNK_FRAMES: usize = 16;
/// Latent channels of the sampler/vocoder contract (`[1, 144, L]`).
pub const LATENT_CHANNELS: usize = 144;
/// Reference tempo constant: predicted seconds are divided by it.
pub const SPEED: f32 = 1.05;
pub const SEED: u64 = 1234;
pub const GUIDANCE: f32 = 3.0;

#[derive(Debug)]
pub struct TeraEngine {
    release: PathBuf,
    text_encoder: Session,
    duration_predictor: Session,
    sampler: Session,
    vocoder: Session,
    /// The single declared output name of each graph (validated at load).
    text_encoder_out: String,
    duration_predictor_out: String,
    sampler_out: String,
    vocoder_out: String,
    indexer: UnicodeIndexer,
}

/// Synthesized utterance: mono f32 chunks at the engine's fixed 44.1 kHz
/// ([`SAMPLE_RATE`]).
pub struct SynthOutput {
    pub chunks: Vec<Vec<f32>>,
}

impl TeraEngine {
    /// Load and verify the pinned release. Fails with `not-installed` reasons
    /// surfaced verbatim on the stdout protocol.
    pub fn load(tts_root: &Path) -> Result<TeraEngine> {
        let started = Instant::now();
        let manifest = Manifest::pinned()?;
        let release = manifest.release_dir(tts_root);
        manifest::check_installed(&manifest, &release)
            .map_err(|e| anyhow!("not-installed: {e}"))?;

        let models = release.join("models");
        let text_encoder = load_session(&models.join("text_encoder.onnx"))?;
        eprintln!(
            "[suflyor-teratts] load stage=text-encoder elapsed_ms={}",
            started.elapsed().as_millis()
        );
        let duration_predictor = load_session(&models.join("duration_predictor.onnx"))?;
        eprintln!(
            "[suflyor-teratts] load stage=duration elapsed_ms={}",
            started.elapsed().as_millis()
        );
        let sampler = load_session(&models.join("sampler_distilled_cfg3_8step.onnx"))?;
        eprintln!(
            "[suflyor-teratts] load stage=sampler elapsed_ms={}",
            started.elapsed().as_millis()
        );
        let vocoder = load_session(&models.join("vocoder.onnx"))?;
        eprintln!(
            "[suflyor-teratts] load stage=vocoder elapsed_ms={}",
            started.elapsed().as_millis()
        );
        let text_encoder_out = sole_declared_output(
            "text_encoder",
            text_encoder.outputs().iter().map(|o| o.name()),
        )?;
        let duration_predictor_out = sole_declared_output(
            "duration_predictor",
            duration_predictor.outputs().iter().map(|o| o.name()),
        )?;
        let sampler_out =
            sole_declared_output("sampler", sampler.outputs().iter().map(|o| o.name()))?;
        let vocoder_out =
            sole_declared_output("vocoder", vocoder.outputs().iter().map(|o| o.name()))?;
        let indexer = UnicodeIndexer::load(&release.join("unicode_indexer.json"))?;

        Ok(TeraEngine {
            release,
            text_encoder,
            duration_predictor,
            sampler,
            vocoder,
            text_encoder_out,
            duration_predictor_out,
            sampler_out,
            vocoder_out,
            indexer,
        })
    }

    /// Synthesize one utterance. `duration_scale` follows the upstream meaning
    /// (>1 = slower). `voice` must be an installed style directory.
    pub fn synthesize(
        &mut self,
        text: &str,
        voice: &str,
        lang: &str,
        duration_scale: f32,
        seed: u64,
    ) -> Result<SynthOutput> {
        let started = Instant::now();
        if !duration_scale.is_finite() || duration_scale <= 0.0 {
            return Err(anyhow!("invalid-rate"));
        }
        let style_ttl = self.load_style(voice, "style_ttl.npy", &[1, 50, 256])?;
        let style_dp = self.load_style(voice, "style_dp.npy", &[1, 8, 16])?;

        let tagged = textnorm::ensure_language_tags(text, lang);
        let model_text =
            textnorm::prepare(&tagged, &self.indexer).map_err(|e| anyhow!("invalid-text: {e}"))?;
        let (text_ids, text_mask) = self
            .indexer
            .batch(&model_text.model_text)
            .map_err(|e| anyhow!("invalid-text: {e}"))?;
        let (duration_ids, duration_mask) = self
            .indexer
            .batch(&model_text.duration_text)
            .map_err(|e| anyhow!("invalid-text: {e}"))?;

        // --- text encoder -------------------------------------------------
        let text_len = text_ids.len();
        let text_ids_t = Tensor::from_array(([1, text_len], text_ids.into_boxed_slice()))
            .map_err(|e| anyhow!("synth: {e}"))?;
        let text_mask_t = Tensor::from_array(([1, 1, text_len], text_mask.into_boxed_slice()))
            .map_err(|e| anyhow!("synth: {e}"))?;
        let style_ttl_t = Tensor::from_array(([1, 50, 256], style_ttl.data.into_boxed_slice()))
            .map_err(|e| anyhow!("synth: {e}"))?;
        let encoder_outputs = self
            .text_encoder
            .run(ort::inputs![
                "text_ids" => &text_ids_t,
                "style_ttl" => &style_ttl_t,
                "text_mask" => &text_mask_t,
            ])
            .map_err(|e| anyhow!("synth: text encoder failed: {e}"))?;
        eprintln!(
            "[suflyor-teratts] synth stage=text-encoder elapsed_ms={}",
            started.elapsed().as_millis()
        );
        let (emb_shape, emb_data) = named_output_f32(&encoder_outputs, &self.text_encoder_out)?;
        validate_tensor_shape(&emb_shape, emb_data.len(), "text_emb")?;

        // --- duration predictor --------------------------------------------
        let dur_len = duration_ids.len();
        let duration_ids_t = Tensor::from_array(([1, dur_len], duration_ids.into_boxed_slice()))
            .map_err(|e| anyhow!("synth: {e}"))?;
        let duration_mask_t =
            Tensor::from_array(([1, 1, dur_len], duration_mask.into_boxed_slice()))
                .map_err(|e| anyhow!("synth: {e}"))?;
        let style_dp_t = Tensor::from_array(([1, 8, 16], style_dp.data.into_boxed_slice()))
            .map_err(|e| anyhow!("synth: {e}"))?;
        let duration_outputs = self
            .duration_predictor
            .run(ort::inputs![
                "text_ids" => &duration_ids_t,
                "style_dp" => &style_dp_t,
                "text_mask" => &duration_mask_t,
            ])
            .map_err(|e| anyhow!("synth: duration predictor failed: {e}"))?;
        eprintln!(
            "[suflyor-teratts] synth stage=duration elapsed_ms={}",
            started.elapsed().as_millis()
        );
        let (dur_shape, dur_data) =
            named_output_f32(&duration_outputs, &self.duration_predictor_out)?;
        validate_tensor_shape(&dur_shape, dur_data.len(), "duration")?;
        let Some(&raw_duration) = dur_data.first() else {
            return Err(anyhow!("synth: duration predictor returned no value"));
        };
        let duration_seconds = raw_duration * duration_scale / SPEED;
        if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
            return Err(anyhow!("synth: non-positive duration"));
        }
        let latent_length = (duration_seconds * SAMPLE_RATE as f32
            / SAMPLES_PER_COMPRESSED_FRAME as f32)
            .ceil()
            .max(1.0) as usize;
        let maximum_samples = (duration_seconds * SAMPLE_RATE as f32).round() as usize;

        // --- distilled 8-step sampler ---------------------------------------
        let mut latent = vec![0.0_f32; LATENT_CHANNELS * latent_length];
        Rng::new(seed).fill_normal_f32(&mut latent);
        let initial_latent_t = Tensor::from_array((
            [1, LATENT_CHANNELS, latent_length],
            latent.into_boxed_slice(),
        ))
        .map_err(|e| anyhow!("synth: {e}"))?;
        let latent_mask_t = Tensor::from_array((
            [1, 1, latent_length],
            vec![1.0_f32; latent_length].into_boxed_slice(),
        ))
        .map_err(|e| anyhow!("synth: {e}"))?;
        let text_emb_t = Tensor::from_array((emb_shape.clone(), emb_data.into_boxed_slice()))
            .map_err(|e| anyhow!("synth: {e}"))?;
        let guidance_t = Tensor::from_array(([1], [GUIDANCE].into_iter().collect::<Box<[f32]>>()))
            .map_err(|e| anyhow!("synth: {e}"))?;
        let sampler_outputs = self
            .sampler
            .run(ort::inputs![
                "initial_latent" => &initial_latent_t,
                "text_emb" => &text_emb_t,
                "style_ttl" => &style_ttl_t,
                "latent_mask" => &latent_mask_t,
                "text_mask" => &text_mask_t,
                "guidance" => &guidance_t,
            ])
            .map_err(|e| anyhow!("synth: sampler failed: {e}"))?;
        eprintln!(
            "[suflyor-teratts] synth stage=sampler frames={} elapsed_ms={}",
            latent_length,
            started.elapsed().as_millis()
        );
        let (latent_shape, latent_out) = named_output_f32(&sampler_outputs, &self.sampler_out)?;
        // Validate the exact [1, 144, L] contract BEFORE the vocoder loop
        // slices `LATENT_CHANNELS * frame` windows out of this buffer.
        validate_latent_output(&latent_shape, latent_out.len(), latent_length)?;

        // --- vocoder: causal overlap-save streaming --------------------------
        let mut chunks: Vec<Vec<f32>> = Vec::new();
        let mut emitted = 0usize;
        let mut start = 0usize;
        while start < latent_length {
            let end = (start + STREAM_CHUNK_FRAMES).min(latent_length);
            let input_start = start.saturating_sub(VOCODER_CONTEXT_FRAMES);
            let latent_window = slice_latent_frames(
                &latent_out,
                LATENT_CHANNELS,
                latent_length,
                input_start,
                end,
            )?;
            let latent_chunk_t = Tensor::from_array((
                [1, LATENT_CHANNELS, end - input_start],
                latent_window.into_boxed_slice(),
            ))
            .map_err(|e| anyhow!("synth: {e}"))?;
            let vocoder_outputs = self
                .vocoder
                .run(ort::inputs!["latent" => &latent_chunk_t])
                .map_err(|e| anyhow!("synth: vocoder failed: {e}"))?;
            let (wav_shape, decoded) = named_output_f32(&vocoder_outputs, &self.vocoder_out)?;
            let discard = (start - input_start) * SAMPLES_PER_COMPRESSED_FRAME;
            let new_samples = (end - start) * SAMPLES_PER_COMPRESSED_FRAME;
            // Validate shape + length BEFORE slicing the overlap-save window.
            validate_vocoder_output(&wav_shape, decoded.len(), discard + new_samples)?;
            let mut chunk = decoded[discard..discard + new_samples].to_vec();
            let remaining = maximum_samples.saturating_sub(emitted);
            if remaining == 0 {
                break;
            }
            chunk.truncate(remaining);
            emitted += chunk.len();
            if !chunk.is_empty() {
                chunks.push(chunk);
            }
            start = end;
        }

        eprintln!(
            "[suflyor-teratts] synth stage=vocoder chunks={} samples={} elapsed_ms={}",
            chunks.len(),
            emitted,
            started.elapsed().as_millis()
        );

        Ok(SynthOutput { chunks })
    }

    fn load_style(&self, voice: &str, file: &str, shape: &[usize]) -> Result<NpyArray> {
        let path = self.release.join("styles").join(voice).join(file);
        if !path.is_file() {
            return Err(anyhow!("unknown-voice"));
        }
        let array = npy::load_f32(&path)?;
        if array.shape != shape {
            return Err(anyhow!("synth: style asset has unexpected shape"));
        }
        Ok(array)
    }
}

/// Copy a `[1, channels, frames]` tensor window while preserving its
/// row-major channel-first layout. A flat contiguous range would treat the
/// tensor as frame-major and feed the vocoder interleaved channel fragments.
fn slice_latent_frames(
    latent: &[f32],
    channels: usize,
    total_frames: usize,
    start: usize,
    end: usize,
) -> Result<Vec<f32>> {
    if channels == 0 || total_frames == 0 || start >= end || end > total_frames {
        return Err(anyhow!("synth: invalid latent frame window"));
    }
    let expected = channels
        .checked_mul(total_frames)
        .ok_or_else(|| anyhow!("synth: latent shape overflow"))?;
    if latent.len() != expected {
        return Err(anyhow!("synth: latent shape/data length mismatch"));
    }
    let window_frames = end - start;
    let mut window = Vec::with_capacity(channels * window_frames);
    for channel in 0..channels {
        let channel_start = channel * total_frames;
        window.extend_from_slice(&latent[channel_start + start..channel_start + end]);
    }
    Ok(window)
}

fn load_session(path: &Path) -> Result<Session> {
    Session::builder()
        .map_err(|e| anyhow!("ort session builder: {e}"))?
        .commit_from_file(path)
        .map_err(|e| anyhow!("load {}: {e}", path.display()))
}

/// Exact declared-output schema of the pinned graphs: each graph returns
/// EXACTLY ONE tensor. Zero or multiple declared outputs are rejected here,
/// at load time, so runtime selection is always a documented name lookup —
/// never positional iteration order, which an ambiguous graph could silently
/// reorder.
fn sole_declared_output<'a>(
    graph: &str,
    names: impl IntoIterator<Item = &'a str>,
) -> Result<String> {
    let mut picked: Option<String> = None;
    for name in names {
        if picked.is_some() {
            return Err(anyhow!("load {graph}: graph declares multiple outputs"));
        }
        picked = Some(name.to_string());
    }
    picked.ok_or_else(|| anyhow!("load {graph}: graph declares no outputs"))
}

/// Extract the graph's single named output as (shape, flat f32 data). In ort
/// rc.13 `try_extract_tensor::<f32>()` yields borrowed `(&Shape, &[f32])`.
/// Negative or non-tensor dims are rejected, never clamped.
fn named_output_f32(
    outputs: &ort::session::SessionOutputs<'_>,
    name: &str,
) -> Result<(Vec<usize>, Vec<f32>)> {
    let Some(value) = outputs.get(name) else {
        return Err(anyhow!("synth: graph returned no output named {name}"));
    };
    let (shape, data) = value
        .try_extract_tensor::<f32>()
        .map_err(|e| anyhow!("synth: unexpected output tensor: {e}"))?;
    // `Shape` derefs to `[i64]`.
    let mut dims = Vec::with_capacity(shape.len());
    for &dim in shape.iter() {
        let dim = usize::try_from(dim)
            .map_err(|_| anyhow!("synth: output shape has a negative dimension"))?;
        dims.push(dim);
    }
    Ok((dims, data.to_vec()))
}

/// Element count implied by a shape: rejects empty shapes, zero dims, and
/// overflowing products, then checks it equals the flat buffer length. Every
/// slice/index below is guarded by this.
fn validate_tensor_shape(shape: &[usize], data_len: usize, what: &str) -> Result<()> {
    let product = shape_product(shape)
        .map_err(|e| anyhow!("synth: {what} shape invalid: {}", e.root_cause()))?;
    if product != data_len {
        return Err(anyhow!("synth: {what} shape/data length mismatch"));
    }
    Ok(())
}

fn shape_product(shape: &[usize]) -> Result<usize> {
    if shape.is_empty() {
        return Err(anyhow!("empty shape"));
    }
    let mut product = 1usize;
    for &dim in shape {
        if dim == 0 {
            return Err(anyhow!("zero dimension"));
        }
        product = product
            .checked_mul(dim)
            .ok_or_else(|| anyhow!("shape product overflow"))?;
    }
    Ok(product)
}

/// Sampler latent contract: exactly `[1, 144, L]` where `L` is the frame
/// count the caller will index — the vocoder loop slices
/// `LATENT_CHANNELS * frame` windows out of this buffer.
fn validate_latent_output(shape: &[usize], data_len: usize, latent_length: usize) -> Result<()> {
    if shape.len() != 3 || shape[0] != 1 || shape[1] != LATENT_CHANNELS || shape[2] != latent_length
    {
        return Err(anyhow!("synth: sampler output shape mismatch"));
    }
    validate_tensor_shape(shape, data_len, "latent")
}

/// Vocoder waveform contract: `[1, S]` with `S` equal to the flat data length
/// and at least as many samples as the overlap-save window about to be sliced.
fn validate_vocoder_output(shape: &[usize], data_len: usize, min_samples: usize) -> Result<()> {
    if shape.len() != 2 || shape[0] != 1 {
        return Err(anyhow!("synth: vocoder output shape mismatch"));
    }
    if shape[1] != data_len {
        return Err(anyhow!("synth: vocoder shape/data length mismatch"));
    }
    if data_len < min_samples {
        return Err(anyhow!("synth: vocoder returned too few samples"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn constants_match_the_reference_release() {
        assert_eq!(SAMPLE_RATE, 44_100);
        assert_eq!(SAMPLES_PER_COMPRESSED_FRAME, 3_072);
        assert_eq!(VOCODER_CONTEXT_FRAMES, 20);
        assert_eq!(STREAM_CHUNK_FRAMES, 16);
        assert_eq!(SPEED, 1.05);
        assert_eq!(SEED, 1234);
        assert_eq!(GUIDANCE, 3.0);
    }

    #[test]
    fn load_fails_with_not_installed_when_dir_absent() {
        let dir = tempfile::tempdir().unwrap();
        let err = TeraEngine::load(dir.path()).unwrap_err();
        assert!(err.to_string().starts_with("not-installed"), "{err}");
    }

    // ===== Hermetic malformed-output schema tests (no model, no ort run) ===

    #[test]
    fn sole_declared_output_rejects_ambiguity() {
        // Exactly one declared output is the pinned contract.
        assert_eq!(
            sole_declared_output("graph", ["text_emb"]).unwrap(),
            "text_emb"
        );
        assert!(sole_declared_output("graph", Vec::<&str>::new()).is_err());
        let err = sole_declared_output("graph", ["a", "b"]).unwrap_err();
        assert!(err.to_string().contains("multiple"), "{err}");
    }

    #[test]
    fn shape_product_rejects_empty_zero_and_overflow() {
        assert_eq!(shape_product(&[1, LATENT_CHANNELS, 8]).unwrap(), 1152);
        assert!(shape_product(&[]).is_err());
        assert!(shape_product(&[1, 0, 8]).is_err());
        assert!(shape_product(&[usize::MAX, 2]).is_err());
    }

    #[test]
    fn tensor_shape_validation_requires_exact_lengths() {
        assert!(validate_tensor_shape(&[1, 3], 3, "x").is_ok());
        assert!(validate_tensor_shape(&[1, 3], 2, "x").is_err()); // short data
        assert!(validate_tensor_shape(&[1, 3], 4, "x").is_err()); // long data
        assert!(validate_tensor_shape(&[], 0, "x").is_err()); // empty shape
        assert!(validate_tensor_shape(&[1, 0], 0, "x").is_err()); // zero dim
    }

    #[test]
    fn latent_output_validation_rejects_malformed_shapes() {
        // Exact contract: [1, 144, L] with data == 144 * L.
        assert!(validate_latent_output(&[1, LATENT_CHANNELS, 4], 576, 4).is_ok());
        assert!(validate_latent_output(&[LATENT_CHANNELS, 4], 576, 4).is_err()); // rank
        assert!(validate_latent_output(&[2, LATENT_CHANNELS, 4], 1152, 4).is_err()); // batch
        assert!(validate_latent_output(&[1, 96, 4], 384, 4).is_err()); // channels
        assert!(validate_latent_output(&[1, LATENT_CHANNELS, 5], 720, 4).is_err()); // L
                                                                                    // The historic panic case: short data must be rejected BEFORE any
                                                                                    // `144 * frame` slice is attempted.
        assert!(validate_latent_output(&[1, LATENT_CHANNELS, 4], 100, 4).is_err());
        assert!(validate_latent_output(&[1, LATENT_CHANNELS, 4], 575, 4).is_err());
    }

    #[test]
    fn vocoder_output_validation_rejects_short_or_misshapen_waveforms() {
        assert!(validate_vocoder_output(&[1, 3072], 3072, 3072).is_ok());
        assert!(validate_vocoder_output(&[1, 6144], 6144, 3072).is_ok()); // extra ok
        assert!(validate_vocoder_output(&[1, 100], 100, 3072).is_err()); // too few
        assert!(validate_vocoder_output(&[1, 3072], 100, 1).is_err()); // shape!=data
        assert!(validate_vocoder_output(&[2, 3072], 3072, 1).is_err()); // batch
        assert!(validate_vocoder_output(&[3072], 3072, 1).is_err()); // rank
    }

    #[test]
    fn latent_frame_window_preserves_channel_first_layout() {
        // [1, 3, 4] flattened as three channel rows.
        let latent = vec![
            10.0, 11.0, 12.0, 13.0, // channel 0
            20.0, 21.0, 22.0, 23.0, // channel 1
            30.0, 31.0, 32.0, 33.0, // channel 2
        ];
        assert_eq!(
            slice_latent_frames(&latent, 3, 4, 1, 3).unwrap(),
            vec![11.0, 12.0, 21.0, 22.0, 31.0, 32.0]
        );
        assert!(slice_latent_frames(&latent, 3, 4, 3, 3).is_err());
        assert!(slice_latent_frames(&latent[..11], 3, 4, 1, 3).is_err());
    }
}
