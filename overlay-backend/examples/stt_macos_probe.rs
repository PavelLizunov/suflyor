//! Bounded macOS GigaAM timing probe.
//!
//! It exercises the production `stt::transcribe_once` path with a caller-owned
//! 16 kHz mono PCM WAV and reports only timing, the post-load accelerator
//! preference, and a known-answer match. Pass a non-sensitive substring to prove the
//! smoke without printing transcript contents.
//!
//! Usage:
//!   cargo run --manifest-path overlay-backend/Cargo.toml --example stt_macos_probe -- \
//!     <cpu|coreml> <model-dir> <wav> <expected-substring> [repeats]

#[cfg(target_os = "macos")]
use anyhow::{bail, Context, Result};
#[cfg(target_os = "macos")]
use overlay_backend::config::SttBackendCfg;
#[cfg(target_os = "macos")]
use std::time::Instant;

#[cfg(target_os = "macos")]
const DEFAULT_REPEATS: usize = 2;
#[cfg(target_os = "macos")]
const MAX_REPEATS: usize = 5;

#[cfg(target_os = "macos")]
fn usage() -> &'static str {
    "usage: stt_macos_probe <cpu|coreml> <model-dir> <wav> <expected-substring> [repeats 1-5]"
}

#[cfg(target_os = "macos")]
fn read_pcm(path: &str) -> Result<(Vec<i16>, f64)> {
    let mut reader = hound::WavReader::open(path).context("open probe WAV")?;
    let spec = reader.spec();
    if spec.channels != 1
        || spec.sample_rate != 16_000
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        bail!("probe WAV must be 16 kHz mono signed 16-bit PCM");
    }
    let samples = reader
        .samples::<i16>()
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("decode probe WAV")?;
    if samples.is_empty() {
        bail!("probe WAV is empty");
    }
    let duration = samples.len() as f64 / f64::from(spec.sample_rate);
    Ok((samples, duration))
}

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let accelerator = args.next().context(usage())?;
    let model_dir = args.next().context(usage())?;
    let wav = args.next().context(usage())?;
    let expected = args.next().context(usage())?;
    let repeats = match args.next() {
        Some(raw) => raw.parse::<usize>().context("repeats must be an integer")?,
        None => DEFAULT_REPEATS,
    };
    if args.next().is_some() || !(1..=MAX_REPEATS).contains(&repeats) {
        bail!(usage());
    }
    let use_coreml = match accelerator.as_str() {
        "cpu" => false,
        "coreml" => true,
        _ => bail!(usage()),
    };
    if expected.trim().is_empty() {
        bail!("expected substring must not be empty");
    }

    let (pcm, audio_seconds) = read_pcm(&wav)?;
    let backend = SttBackendCfg::Gigaam { model_dir };
    overlay_backend::stt::configure_gigaam_accelerator(use_coreml);
    overlay_backend::stt::reset_gigaam_cache();

    println!(
        "probe requested_accelerator={accelerator} audio_seconds={audio_seconds:.3} repeats={repeats}"
    );
    for run in 1..=repeats {
        let started = Instant::now();
        let result = overlay_backend::stt::transcribe_once(&backend, &pcm, None, None).await;
        let elapsed = started.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        let rtf = elapsed.as_secs_f64() / audio_seconds;
        let accelerator_preference = transcribe_rs::get_ort_accelerator();
        let text = match result {
            Ok(text) => text,
            Err(error) => {
                println!(
                    "run={run} phase={} elapsed_ms={elapsed_ms:.3} rtf={rtf:.4} \
                     accelerator_preference={accelerator_preference:?} status=error",
                    if run == 1 { "cold" } else { "warm" },
                );
                return Err(error).context("STT probe transcription failed");
            }
        };
        let expect_match = text.to_lowercase().contains(&expected.to_lowercase());
        println!(
            "run={run} phase={} elapsed_ms={elapsed_ms:.3} rtf={rtf:.4} \
             accelerator_preference={accelerator_preference:?} expect_match={expect_match}",
            if run == 1 { "cold" } else { "warm" },
        );
        if !expect_match {
            bail!("known-answer substring was not recognized");
        }
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("stt_macos_probe: unsupported outside macOS");
    std::process::exit(2);
}
