//! Dedicated-worker acceptance: managed model installation and real PipeWire speech.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use overlay_backend::{audio, config, health, local_ai, recorder, stt};
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Duration, Instant};

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name} on the isolated native worker"))
}
fn assert_speech(text: &str) {
    let text = text.to_lowercase();
    assert!(
        text.contains("проверка") && text.contains("связи"),
        "unexpected public fixture transcript: {text}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "downloads a real GigaAM model; requires isolated PipeWire sink/source and a public speech fixture"]
async fn managed_gigaam_transcribes_both_native_streams_and_records() {
    let root = PathBuf::from(required("SUFLYOR_TEST_MODEL_ROOT"));
    let wav = PathBuf::from(required("SUFLYOR_TEST_SPEECH_WAV"));
    let sink = required("SUFLYOR_TEST_AUDIO_SINK");
    let source = required("SUFLYOR_TEST_AUDIO_SOURCE");
    let install_root = root.clone();
    let result = tokio::task::spawn_blocking(move || {
        local_ai::install(
            &local_ai::InstallOptions {
                root: install_root,
                force_cpu: true,
                download_only: true,
                skip_llama: true,
                skip_whisper: true,
                skip_gigaam: false,
                ..Default::default()
            },
            &AtomicBool::new(false),
            &|_| {},
        )
    })
    .await
    .unwrap()
    .unwrap();
    assert!(
        result.servers.is_empty(),
        "download-only installation started a server"
    );
    assert!(
        !result.stt_gigaam_dir.is_empty(),
        "managed STT install did not produce a model"
    );
    let mut cfg = config::Config::defaults();
    cfg.ai_provider = "cloud".into();
    cfg.vision_provider = "off".into();
    cfg.stt_provider = "cloud".into();
    local_ai::apply_download_result(&mut cfg, &result);
    assert_eq!(cfg.ai_provider, "cloud");
    assert_eq!(cfg.vision_provider, "off");
    assert_eq!(cfg.stt_provider, "cloud");
    let backend = config::SttBackendCfg::Gigaam {
        model_dir: result.stt_gigaam_dir,
    };
    stt::configure_gigaam_accelerator(false);
    let mut reader = hound::WavReader::open(&wav).unwrap();
    assert_eq!(reader.spec().sample_rate, 16000);
    assert_eq!(reader.spec().channels, 1);
    let samples = reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_speech(
        &stt::transcribe_once(&backend, &samples, Some("ru"), None)
            .await
            .unwrap(),
    );

    let (mut captured, capture) = audio::start_capture(Some(source), Some(sink.clone())).unwrap();
    let (tx, rx) = tokio::sync::mpsc::channel(128);
    let mut transcripts = stt::spawn(
        rx,
        backend,
        Some("ru".into()),
        None,
        Arc::new(health::HealthSignals::default()),
    );
    let temporary = tempfile::tempdir().unwrap();
    let recording = recorder::SessionRecorder::start_in(temporary.path().join("speech")).unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut player = std::process::Command::new("paplay")
        .args(["--device", &sink])
        .arg(&wav)
        .spawn()
        .unwrap();
    let start = Instant::now();
    let mut counts = [0usize; 2];
    while start.elapsed() < Duration::from_secs(5) {
        if let Ok(chunk) = tokio::time::timeout(Duration::from_millis(250), captured.recv()).await {
            let Some(chunk) = chunk else { break };
            let index = usize::from(chunk.source == audio::AudioSource::System);
            counts[index] += chunk.pcm_i16.len();
            recording.feed(&chunk);
            tx.send(chunk).await.unwrap();
        }
    }
    assert!(player.wait().unwrap().success());
    drop(capture);
    drop(tx);
    drop(recording);
    assert!(
        counts.iter().all(|n| *n >= 32000),
        "missing native stream: {counts:?}"
    );
    let mut seen = [false; 2];
    let deadline = Instant::now() + Duration::from_secs(60);
    while !seen.iter().all(|v| *v) && Instant::now() < deadline {
        let event = tokio::time::timeout(Duration::from_secs(30), transcripts.recv())
            .await
            .unwrap()
            .unwrap();
        assert_speech(&event.text);
        seen[usize::from(event.source == audio::AudioSource::System)] = true;
    }
    assert!(
        seen.iter().all(|v| *v),
        "speech did not reach both transcript paths"
    );
    for name in ["mic.wav", "system.wav"] {
        let recording = hound::WavReader::open(temporary.path().join("speech").join(name)).unwrap();
        assert!(recording.duration() >= 32000, "short recorded channel");
    }
    stt::reset_gigaam_cache();
}
