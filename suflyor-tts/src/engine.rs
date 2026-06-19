//! Neural TTS engine (sidecar): sherpa-onnx VITS wrapper + voice discovery +
//! pure text helpers. Mirrors the logic that briefly lived in overlay-backend
//! before the two-onnxruntime crash forced it into this separate process.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

/// Loaded sherpa-onnx VITS engine for one voice.
pub struct NeuralEngine {
    tts: sherpa_onnx::OfflineTts,
    sample_rate: u32,
}

impl NeuralEngine {
    pub fn load(model: &Path, tokens: &Path, data_dir: Option<&Path>) -> Result<NeuralEngine> {
        if !model.is_file() {
            return Err(anyhow!("tts model not found"));
        }
        if !tokens.is_file() {
            return Err(anyhow!("tts tokens not found"));
        }
        let vits = sherpa_onnx::OfflineTtsVitsModelConfig {
            model: Some(model.to_string_lossy().to_string()),
            tokens: Some(tokens.to_string_lossy().to_string()),
            data_dir: data_dir.map(|d| d.to_string_lossy().to_string()),
            ..Default::default()
        };
        let model_cfg = sherpa_onnx::OfflineTtsModelConfig {
            vits,
            num_threads: 1,
            debug: false,
            provider: Some("cpu".to_string()),
            ..Default::default()
        };
        let cfg = sherpa_onnx::OfflineTtsConfig {
            model: model_cfg,
            ..Default::default()
        };
        let tts = sherpa_onnx::OfflineTts::create(&cfg)
            .ok_or_else(|| anyhow!("failed to create OfflineTts (model not loadable)"))?;
        let sample_rate = tts.sample_rate().max(0) as u32;
        Ok(NeuralEngine { tts, sample_rate })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Synthesize one chunk → mono f32 samples. Empty for blank text.
    pub fn synth(&self, text: &str, speed: f32, sid: i32) -> Result<Vec<f32>> {
        let clean = text::sanitize(text);
        if clean.trim().is_empty() {
            return Ok(Vec::new());
        }
        let gen = sherpa_onnx::GenerationConfig {
            sid,
            speed: if speed.is_finite() && speed > 0.0 {
                speed
            } else {
                1.0
            },
            ..Default::default()
        };
        let audio = self
            .tts
            .generate_with_config(&clean, &gen, None::<fn(&[f32], f32) -> bool>)
            .ok_or_else(|| anyhow!("synthesis returned no audio"))?;
        Ok(audio.samples().to_vec())
    }
}

/// Map a user rate (−10..+10) to a synthesis speed multiplier (−10→0.5×, 0→1×,
/// +10→2×). Out-of-range clamped.
pub fn rate_to_speed(rate: i32) -> f32 {
    let r = rate.clamp(-10, 10) as f32;
    2.0_f32.powf(r / 10.0)
}

/// `%APPDATA%\suflyor\tts` — where voice models are installed.
pub fn tts_root() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("suflyor").join("tts"))
}

/// A selectable voice: `id` is the model dir name, `name` the display name.
#[derive(Debug, Clone)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
}

/// Scan `tts_dir` for installed voices (subdir with a `*.onnx` + `tokens.txt`).
pub fn scan_voices(tts_dir: &Path) -> Vec<VoiceInfo> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(tts_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if find_onnx(&path).is_some() && path.join("tokens.txt").is_file() {
            out.push(VoiceInfo {
                id: name.to_string(),
                name: friendly_name(name),
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Load the engine for `voice_dir`. espeak-ng-data (Piper) is checked inside the
/// voice dir first, then a shared copy at the tts root; absent → char-tokenized.
pub fn load_voice(tts_dir: &Path, voice_dir: &str) -> Result<NeuralEngine> {
    let vd = tts_dir.join(voice_dir);
    let onnx =
        find_onnx(&vd).ok_or_else(|| anyhow!("no .onnx model in voice dir '{voice_dir}'"))?;
    let tokens = vd.join("tokens.txt");
    let per_voice = vd.join("espeak-ng-data");
    let shared = tts_dir.join("espeak-ng-data");
    let data_dir = if per_voice.is_dir() {
        Some(per_voice)
    } else if shared.is_dir() {
        Some(shared)
    } else {
        None
    };
    NeuralEngine::load(&onnx, &tokens, data_dir.as_deref())
}

fn find_onnx(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("onnx") {
            return Some(p);
        }
    }
    None
}

/// Choose which voice id to load: configured if installed, else Irina → any
/// Piper → any Russian → first installed.
pub fn pick_voice_id(voices: &[VoiceInfo], configured: &str) -> Option<String> {
    if !configured.is_empty() && voices.iter().any(|v| v.id == configured) {
        return Some(configured.to_string());
    }
    for pref in ["irina", "piper", "ru_ru", "ru-ru", "rus"] {
        if let Some(v) = voices
            .iter()
            .find(|v| format!("{} {}", v.id, v.name).to_lowercase().contains(pref))
        {
            return Some(v.id.clone());
        }
    }
    voices.first().map(|v| v.id.clone())
}

fn friendly_name(dir: &str) -> String {
    let d = dir.to_lowercase();
    if d.contains("irina") {
        "Ирина (ж)".to_string()
    } else if d.contains("ruslan") {
        "Руслан (м)".to_string()
    } else if d.contains("dmitri") {
        "Дмитрий (м)".to_string()
    } else if d.contains("denis") {
        "Денис (м)".to_string()
    } else if d.contains("mms") {
        "MMS (рус)".to_string()
    } else {
        dir.to_string()
    }
}

/// Pure text helpers (chunking + sanitization).
pub mod text {
    pub const MAX_CHUNK_CHARS: usize = 350;

    pub fn sanitize(text: &str) -> String {
        text.chars()
            .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
            .collect()
    }

    pub fn chunk_text(text: &str) -> Vec<String> {
        let mut chunks: Vec<String> = Vec::new();
        let mut cur = String::new();
        for sentence in split_sentences(text) {
            let s_len = sentence.chars().count();
            if s_len > MAX_CHUNK_CHARS {
                push_trimmed(&mut chunks, std::mem::take(&mut cur));
                for piece in hard_split(&sentence, MAX_CHUNK_CHARS) {
                    push_trimmed(&mut chunks, piece);
                }
                continue;
            }
            if cur.chars().count() + s_len > MAX_CHUNK_CHARS && !cur.is_empty() {
                push_trimmed(&mut chunks, std::mem::take(&mut cur));
            }
            cur.push_str(&sentence);
        }
        push_trimmed(&mut chunks, cur);
        chunks
    }

    fn push_trimmed(out: &mut Vec<String>, s: String) {
        let t = s.trim();
        if !t.is_empty() {
            out.push(t.to_string());
        }
    }

    fn split_sentences(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        for ch in text.chars() {
            cur.push(ch);
            if matches!(ch, '.' | '!' | '?' | '…' | '\n' | ';') {
                out.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
        out
    }

    fn hard_split(s: &str, max: usize) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        for word in s.split_whitespace() {
            if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > max {
                out.push(std::mem::take(&mut cur));
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
        }
        if !cur.is_empty() {
            out.push(cur);
        }
        out
    }
}
