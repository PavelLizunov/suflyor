//! Bounded macOS GigaAM memory-lifecycle probe.
//!
//! It exercises the production one-shot and live STT paths without printing
//! audio, transcript text, or caller paths. An external supervisor samples RSS,
//! threads, and file descriptors after each flushed `phase=` marker.
//!
//! Usage:
//!   cargo run --manifest-path overlay-backend/Cargo.toml \
//!     --example stt_macos_lifecycle_probe -- \
//!     <cpu|coreml> <model-dir> <wav> <expected-substring>

#[cfg(target_os = "macos")]
use anyhow::{bail, Context, Result};
#[cfg(target_os = "macos")]
use overlay_backend::audio::{AudioChunk, AudioSource};
#[cfg(target_os = "macos")]
use overlay_backend::config::SttBackendCfg;
#[cfg(target_os = "macos")]
use overlay_backend::stt::TranscriptEvent;
#[cfg(target_os = "macos")]
use std::io::Write as _;
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::time::Duration;
#[cfg(target_os = "macos")]
use tokio::sync::mpsc;

#[cfg(target_os = "macos")]
const LIVE_CYCLES: usize = 5;
#[cfg(target_os = "macos")]
const PHASE_SETTLE: Duration = Duration::from_millis(1200);
#[cfg(target_os = "macos")]
const TRANSCRIPT_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(target_os = "macos")]
const PIPELINE_STOP_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn usage() -> &'static str {
    "usage: stt_macos_lifecycle_probe <cpu|coreml> <model-dir> <wav> <expected-substring>"
}

#[cfg(target_os = "macos")]
fn read_pcm(path: &str) -> Result<Vec<i16>> {
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
    Ok(samples)
}

#[cfg(target_os = "macos")]
async fn phase(name: &str) -> Result<()> {
    println!(
        "phase={name} active_accelerator={:?}",
        transcribe_rs::get_ort_accelerator()
    );
    std::io::stdout().flush().context("flush phase marker")?;
    tokio::time::sleep(PHASE_SETTLE).await;
    Ok(())
}

#[cfg(target_os = "macos")]
fn transcript_matches(event: &TranscriptEvent, expected: &str) -> bool {
    event.text.to_lowercase().contains(&expected.to_lowercase())
}

#[cfg(target_os = "macos")]
async fn start_live(
    backend: &SttBackendCfg,
    pcm: &[i16],
    expected: &str,
) -> Result<(
    mpsc::Sender<AudioChunk>,
    mpsc::Receiver<TranscriptEvent>,
)> {
    let (audio_tx, audio_rx) = mpsc::channel(4);
    let health = Arc::new(overlay_backend::health::HealthSignals::default());
    let mut transcript_rx = overlay_backend::stt::spawn(audio_rx, backend.clone(), None, None, health);
    let speech_ms = (pcm.len() as u64 * 1000) / 16_000;
    audio_tx
        .send(AudioChunk {
            source: AudioSource::Mic,
            pcm_i16: pcm.to_vec(),
            timestamp_ms: speech_ms,
        })
        .await
        .context("send probe speech")?;
    audio_tx
        .send(AudioChunk {
            source: AudioSource::Mic,
            pcm_i16: vec![0; 16_000],
            timestamp_ms: speech_ms + 1000,
        })
        .await
        .context("send probe silence")?;
    let event = tokio::time::timeout(TRANSCRIPT_TIMEOUT, transcript_rx.recv())
        .await
        .context("live transcript timeout")?
        .context("live STT pipeline closed without a transcript")?;
    if !transcript_matches(&event, expected) {
        bail!("known-answer substring was not recognized by live STT");
    }
    Ok((audio_tx, transcript_rx))
}

#[cfg(target_os = "macos")]
async fn stop_live(
    audio_tx: mpsc::Sender<AudioChunk>,
    mut transcript_rx: mpsc::Receiver<TranscriptEvent>,
) -> Result<()> {
    drop(audio_tx);
    tokio::time::timeout(PIPELINE_STOP_TIMEOUT, async move {
        while transcript_rx.recv().await.is_some() {}
    })
    .await
    .context("live STT pipeline did not stop")?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let accelerator = args.next().context(usage())?;
    let model_dir = args.next().context(usage())?;
    let wav = args.next().context(usage())?;
    let expected = args.next().context(usage())?;
    if args.next().is_some() || expected.trim().is_empty() {
        bail!(usage());
    }
    let use_coreml = match accelerator.as_str() {
        "cpu" => false,
        "coreml" => true,
        _ => bail!(usage()),
    };

    let pcm = read_pcm(&wav)?;
    let backend = SttBackendCfg::Gigaam { model_dir };
    overlay_backend::stt::configure_gigaam_accelerator(use_coreml);
    overlay_backend::stt::reset_gigaam_cache();
    println!("probe requested_accelerator={accelerator} live_cycles={LIVE_CYCLES}");
    phase("baseline").await?;

    let one_shot = overlay_backend::stt::transcribe_once(&backend, &pcm, None, None).await?;
    if !one_shot.to_lowercase().contains(&expected.to_lowercase()) {
        bail!("known-answer substring was not recognized by one-shot STT");
    }
    phase("adhoc_loaded").await?;

    let (audio_tx, transcript_rx) = start_live(&backend, &pcm, &expected).await?;
    phase("adhoc_plus_live").await?;
    stop_live(audio_tx, transcript_rx).await?;
    phase("adhoc_after_live_stop").await?;

    overlay_backend::stt::reset_gigaam_cache();
    phase("cache_reset").await?;

    for cycle in 1..=LIVE_CYCLES {
        let (audio_tx, transcript_rx) = start_live(&backend, &pcm, &expected).await?;
        stop_live(audio_tx, transcript_rx).await?;
        phase(&format!("live_cycle_{cycle}_stopped")).await?;
    }
    println!("probe_status=ok");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("stt_macos_lifecycle_probe: unsupported outside macOS");
    std::process::exit(2);
}
