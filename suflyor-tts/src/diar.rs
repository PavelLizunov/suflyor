//! Offline speaker diarization — the `suflyor-tts diarize` subcommand (D1).
//!
//! Runs sherpa-onnx `OfflineSpeakerDiarization` (pyannote segmentation + WeSpeaker
//! embeddings + agglomerative clustering) over ONE recorded `system.wav` and prints
//! its speaker segments as JSON on stdout, then exits. It lives in THIS sidecar (not
//! overlay-backend) for the same reason TTS does: sherpa-onnx links its own
//! onnxruntime, which crashes if statically linked alongside the app's `ort`/GigaAM
//! STT. A `diarize` run is a SEPARATE short-lived process from the read-aloud stdin
//! loop, so the two never share an address space.
//!
//! CLI: `suflyor-tts diarize <system.wav> --seg <seg.onnx> --emb <emb.onnx>
//!       [--num-speakers N] [--threshold T]`
//! stdout (success): `{"num_speakers":N,"segments":[{"s":ms,"e":ms,"sp":i},...]}`

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use sherpa_onnx::{
    FastClusteringConfig, OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig,
    OfflineSpeakerSegmentationModelConfig, OfflineSpeakerSegmentationPyannoteModelConfig,
    SpeakerEmbeddingExtractorConfig, Wave,
};

/// One diarized span: `[start_ms, end_ms)` attributed to speaker index `sp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub sp: i32,
}

/// CLI entry for the `diarize` subcommand. Prints the JSON contract to stdout on
/// success (the reason to stderr + a nonzero code on failure). Returns the process
/// exit code.
pub fn run_cli(args: &[String]) -> i32 {
    match run(args) {
        Ok(json) => {
            println!("{json}");
            0
        }
        Err(e) => {
            eprintln!("[suflyor-diar] {e:#}");
            1
        }
    }
}

fn run(args: &[String]) -> Result<String> {
    let a = parse_args(args)?;
    let (n, segs) = diarize(&a.wav, &a.seg, &a.emb, a.num_speakers, a.threshold)?;
    Ok(to_json(n, &segs))
}

struct Args {
    wav: PathBuf,
    seg: PathBuf,
    emb: PathBuf,
    /// > 0 forces the speaker count; ≤ 0 = auto (threshold clustering).
    num_speakers: i32,
    /// > 0 overrides the clustering threshold; ≤ 0 = the binding default (0.5).
    threshold: f32,
}

const USAGE: &str = "usage: suflyor-tts diarize <system.wav> --seg <seg.onnx> --emb <emb.onnx> [--num-speakers N] [--threshold T]";

fn parse_args(args: &[String]) -> Result<Args> {
    let mut wav: Option<PathBuf> = None;
    let mut seg: Option<PathBuf> = None;
    let mut emb: Option<PathBuf> = None;
    let mut num_speakers = 0i32;
    let mut threshold = 0.0f32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seg" => seg = Some(PathBuf::from(next_val(args, &mut i, "--seg")?)),
            "--emb" => emb = Some(PathBuf::from(next_val(args, &mut i, "--emb")?)),
            "--num-speakers" => {
                num_speakers = next_val(args, &mut i, "--num-speakers")?
                    .parse()
                    .map_err(|_| anyhow!("--num-speakers expects an integer"))?;
            }
            "--threshold" => {
                threshold = next_val(args, &mut i, "--threshold")?
                    .parse()
                    .map_err(|_| anyhow!("--threshold expects a number"))?;
            }
            other if wav.is_none() && !other.starts_with("--") => {
                wav = Some(PathBuf::from(other));
            }
            other => bail!("unexpected argument '{other}'\n{USAGE}"),
        }
        i += 1;
    }
    Ok(Args {
        wav: wav.ok_or_else(|| anyhow!("missing <system.wav>\n{USAGE}"))?,
        seg: seg.ok_or_else(|| anyhow!("missing --seg <seg.onnx>\n{USAGE}"))?,
        emb: emb.ok_or_else(|| anyhow!("missing --emb <emb.onnx>\n{USAGE}"))?,
        num_speakers,
        threshold,
    })
}

/// Consume the value after a flag at `args[*i]`, advancing `i` onto it (the outer
/// loop's `+= 1` then steps past it). Rejects another flag as the value so a typo'd
/// `--seg --emb e.onnx` names the real mistake instead of a confusing "missing --emb".
fn next_val<'a>(args: &'a [String], i: &mut usize, flag: &str) -> Result<&'a str> {
    *i += 1;
    let v = args
        .get(*i)
        .map(String::as_str)
        .ok_or_else(|| anyhow!("{flag} expects a value"))?;
    if v.starts_with("--") {
        bail!("{flag} expects a value, got flag '{v}'");
    }
    Ok(v)
}

/// Diarize a 16 kHz mono WAV. Returns `(num_speakers, segments sorted by start)`.
fn diarize(
    wav: &Path,
    seg: &Path,
    emb: &Path,
    num_speakers: i32,
    threshold: f32,
) -> Result<(i32, Vec<Segment>)> {
    if !seg.is_file() {
        bail!("segmentation model not found: {}", seg.display());
    }
    if !emb.is_file() {
        bail!("embedding model not found: {}", emb.display());
    }
    let cfg = OfflineSpeakerDiarizationConfig {
        segmentation: OfflineSpeakerSegmentationModelConfig {
            pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
                model: Some(seg.to_string_lossy().into_owned()),
            },
            num_threads: 4,
            debug: false,
            provider: Some("cpu".to_string()),
        },
        embedding: SpeakerEmbeddingExtractorConfig {
            model: Some(emb.to_string_lossy().into_owned()),
            num_threads: 4,
            debug: false,
            provider: Some("cpu".to_string()),
        },
        clustering: FastClusteringConfig {
            num_clusters: if num_speakers > 0 { num_speakers } else { -1 },
            threshold: if threshold > 0.0 { threshold } else { 0.5 },
        },
        min_duration_on: 0.3,
        min_duration_off: 0.5,
    };
    let sd = OfflineSpeakerDiarization::create(&cfg)
        .ok_or_else(|| anyhow!("failed to create diarizer (models not loadable)"))?;

    let wav_str = wav
        .to_str()
        .ok_or_else(|| anyhow!("wav path is not valid UTF-8: {}", wav.display()))?;
    let wave = Wave::read(wav_str).ok_or_else(|| anyhow!("cannot read wav: {}", wav.display()))?;
    let expected = sd.sample_rate();
    if wave.sample_rate() != expected {
        bail!(
            "wav is {} Hz but the model expects {} Hz",
            wave.sample_rate(),
            expected
        );
    }
    let samples = wave.samples();
    if samples.is_empty() {
        bail!("wav has no samples");
    }
    let result = sd
        .process(samples)
        .ok_or_else(|| anyhow!("diarization produced no result"))?;
    let n = result.num_speakers();
    let segments = result
        .sort_by_start_time()
        .into_iter()
        .map(|s| Segment {
            start_ms: (f64::from(s.start) * 1000.0) as i64,
            end_ms: (f64::from(s.end) * 1000.0) as i64,
            sp: s.speaker,
        })
        .collect();
    let segments = postprocess(segments);
    Ok((n, segments))
}

/// Smooth short speaker flicker and join same-speaker spans separated by a small gap.
fn postprocess(segments: Vec<Segment>) -> Vec<Segment> {
    const MERGE_GAP_MS: i64 = 600;
    const FRAGMENT_MS: i64 = 400;
    const ATTACH_GAP_MS: i64 = 300;

    let mut out: Vec<Segment> = Vec::with_capacity(segments.len());
    for segment in segments {
        if segment.end_ms - segment.start_ms < FRAGMENT_MS {
            if let Some(previous) = out.last_mut() {
                if segment.start_ms - previous.end_ms < ATTACH_GAP_MS {
                    previous.end_ms = previous.end_ms.max(segment.end_ms);
                }
            }
            continue;
        }
        if let Some(previous) = out.last_mut() {
            if previous.sp == segment.sp && segment.start_ms - previous.end_ms < MERGE_GAP_MS {
                previous.end_ms = previous.end_ms.max(segment.end_ms);
                continue;
            }
        }
        out.push(segment);
    }
    out
}

/// Serialize as the stdout JSON contract. Hand-rolled (the sidecar has no serde;
/// every field is an integer, so there is nothing to escape). The backend client
/// parses it with `serde_json`.
fn to_json(num_speakers: i32, segments: &[Segment]) -> String {
    let mut s = String::from("{\"num_speakers\":");
    s.push_str(&num_speakers.to_string());
    s.push_str(",\"segments\":[");
    for (idx, seg) in segments.iter().enumerate() {
        if idx > 0 {
            s.push(',');
        }
        s.push_str("{\"s\":");
        s.push_str(&seg.start_ms.to_string());
        s.push_str(",\"e\":");
        s.push_str(&seg.end_ms.to_string());
        s.push_str(",\"sp\":");
        s.push_str(&seg.sp.to_string());
        s.push('}');
    }
    s.push_str("]}");
    s
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn to_json_matches_the_contract() {
        assert_eq!(to_json(0, &[]), "{\"num_speakers\":0,\"segments\":[]}");
        let segs = vec![
            Segment {
                start_ms: 0,
                end_ms: 1500,
                sp: 0,
            },
            Segment {
                start_ms: 1500,
                end_ms: 4200,
                sp: 1,
            },
        ];
        assert_eq!(
            to_json(2, &segs),
            "{\"num_speakers\":2,\"segments\":[{\"s\":0,\"e\":1500,\"sp\":0},{\"s\":1500,\"e\":4200,\"sp\":1}]}"
        );
    }

    #[test]
    fn parse_args_reads_positional_and_flags() {
        let a = parse_args(&[
            "sys.wav".into(),
            "--seg".into(),
            "s.onnx".into(),
            "--emb".into(),
            "e.onnx".into(),
            "--num-speakers".into(),
            "4".into(),
            "--threshold".into(),
            "0.6".into(),
        ])
        .unwrap();
        assert_eq!(a.wav, PathBuf::from("sys.wav"));
        assert_eq!(a.seg, PathBuf::from("s.onnx"));
        assert_eq!(a.emb, PathBuf::from("e.onnx"));
        assert_eq!(a.num_speakers, 4);
        assert!((a.threshold - 0.6).abs() < 1e-6);
    }

    #[test]
    fn parse_args_requires_wav_seg_emb() {
        // missing wav / seg / emb each error
        assert!(parse_args(&["--seg".into(), "s".into(), "--emb".into(), "e".into()]).is_err());
        assert!(parse_args(&["w".into(), "--emb".into(), "e".into()]).is_err());
        assert!(parse_args(&["w".into(), "--seg".into(), "s".into()]).is_err());
    }

    #[test]
    fn parse_args_rejects_dangling_flag_and_extra_positional() {
        assert!(parse_args(&["w".into(), "--seg".into()]).is_err()); // no value
        assert!(parse_args(&[
            "w".into(),
            "extra".into(),
            "--seg".into(),
            "s".into(),
            "--emb".into(),
            "e".into()
        ])
        .is_err()); // second positional
    }

    #[test]
    fn parse_args_rejects_flag_as_value_and_bad_numbers() {
        // a flag where a value belongs
        assert!(parse_args(&["w".into(), "--seg".into(), "--emb".into(), "e".into()]).is_err());
        // non-numeric --num-speakers / --threshold
        let base = ["w", "--seg", "s", "--emb", "e"];
        let mut count = base.map(String::from).to_vec();
        count.extend(["--num-speakers".into(), "x".into()]);
        assert!(parse_args(&count).is_err());
        let mut thr = base.map(String::from).to_vec();
        thr.extend(["--threshold".into(), "nope".into()]);
        assert!(parse_args(&thr).is_err());
    }

    fn seg(start_ms: i64, end_ms: i64, sp: i32) -> Segment {
        Segment {
            start_ms,
            end_ms,
            sp,
        }
    }

    #[test]
    fn postprocess_merges_same_speaker_across_short_gap() {
        assert_eq!(
            postprocess(vec![seg(0, 1_000, 0), seg(1_500, 2_500, 0)]),
            vec![seg(0, 2_500, 0)]
        );
    }

    #[test]
    fn postprocess_attaches_short_fragment_to_previous() {
        assert_eq!(
            postprocess(vec![
                seg(0, 1_000, 0),
                seg(1_100, 1_350, 1),
                seg(1_400, 2_000, 0),
            ]),
            vec![seg(0, 2_000, 0)]
        );
    }

    #[test]
    fn postprocess_drops_isolated_short_fragment() {
        assert_eq!(
            postprocess(vec![seg(0, 500, 0), seg(1_000, 1_200, 1)]),
            vec![seg(0, 500, 0)]
        );
    }

    #[test]
    fn postprocess_keeps_sorted_nonoverlapping_segments() {
        let input = vec![seg(0, 500, 0), seg(700, 1_200, 1), seg(2_000, 2_600, 1)];
        assert_eq!(postprocess(input.clone()), input);
    }
}
