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

use std::path::{Path, PathBuf};

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
        let manifest = Manifest::pinned()?;
        let release = manifest.release_dir(tts_root);
        manifest::check_installed(&manifest, &release)
            .map_err(|e| anyhow!("not-installed: {e}"))?;

        let models = release.join("models");
        let text_encoder = load_session(&models.join("text_encoder.onnx"))?;
        let duration_predictor = load_session(&models.join("duration_predictor.onnx"))?;
        let sampler = load_session(&models.join("sampler_distilled_cfg3_8step.onnx"))?;
        let vocoder = load_session(&models.join("vocoder.onnx"))?;
        let indexer = UnicodeIndexer::load(&release.join("unicode_indexer.json"))?;

        Ok(TeraEngine {
            release,
            text_encoder,
            duration_predictor,
            sampler,
            vocoder,
            indexer,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    pub fn voices(&self) -> Vec<String> {
        manifest::installed_voices(&self.release)
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
        let (emb_shape, emb_data) = first_output_f32(&encoder_outputs)?;

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
        let (_dur_shape, dur_data) = first_output_f32(&duration_outputs)?;
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
        let mut latent = vec![0.0_f32; 144 * latent_length];
        Rng::new(seed).fill_normal_f32(&mut latent);
        let initial_latent_t =
            Tensor::from_array(([1, 144, latent_length], latent.into_boxed_slice()))
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
        let (_latent_shape, latent_out) = first_output_f32(&sampler_outputs)?;

        // --- vocoder: causal overlap-save streaming --------------------------
        let mut chunks: Vec<Vec<f32>> = Vec::new();
        let mut emitted = 0usize;
        let mut start = 0usize;
        while start < latent_length {
            let end = (start + STREAM_CHUNK_FRAMES).min(latent_length);
            let input_start = start.saturating_sub(VOCODER_CONTEXT_FRAMES);
            let slice = &latent_out[144 * input_start..144 * end];
            let latent_chunk_t = Tensor::from_array((
                [1, 144, end - input_start],
                slice.to_vec().into_boxed_slice(),
            ))
            .map_err(|e| anyhow!("synth: {e}"))?;
            let vocoder_outputs = self
                .vocoder
                .run(ort::inputs!["latent" => &latent_chunk_t])
                .map_err(|e| anyhow!("synth: vocoder failed: {e}"))?;
            let (_wav_shape, decoded) = first_output_f32(&vocoder_outputs)?;
            let discard = (start - input_start) * SAMPLES_PER_COMPRESSED_FRAME;
            let new_samples = (end - start) * SAMPLES_PER_COMPRESSED_FRAME;
            if decoded.len() < discard + new_samples {
                return Err(anyhow!("synth: vocoder returned too few samples"));
            }
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

fn load_session(path: &Path) -> Result<Session> {
    Session::builder()
        .map_err(|e| anyhow!("ort session builder: {e}"))?
        .commit_from_file(path)
        .map_err(|e| anyhow!("load {}: {e}", path.display()))
}

/// Extract the first output tensor as (shape, flat f32 data). In ort rc.13
/// `try_extract_tensor::<f32>()` yields borrowed `(&Shape, &[f32])`.
fn first_output_f32(outputs: &ort::session::SessionOutputs) -> Result<(Vec<usize>, Vec<f32>)> {
    let Some((_, value)) = outputs.iter().next() else {
        return Err(anyhow!("synth: graph returned no outputs"));
    };
    let (shape, data) = value
        .try_extract_tensor::<f32>()
        .map_err(|e| anyhow!("synth: unexpected output tensor: {e}"))?;
    // `Shape` derefs to `[i64]`.
    let dims = shape.iter().map(|&d| d.max(0) as usize).collect();
    Ok((dims, data.to_vec()))
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
}
