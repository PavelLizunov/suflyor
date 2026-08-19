//! suflyor-teratts — experimental TeraTTSv2 read-aloud sidecar.
//!
//! Reads one command per line on stdin (the suflyor-tts protocol plus the
//! additive LANG command) and synthesizes + plays speech through the pinned
//! TeraSpace/TeraTTSv2 ONNX graphs (ort) at 44.1 kHz. Diarization and the
//! Piper fallback stay in suflyor-tts — two ONNX runtimes never share a
//! process.
//!
//! Protocol (stdin, one per line):
//!   VOICE <id>           select Tera voice style (e.g. ru_f1)
//!   RATE <-10..10>       read rate (maps to duration_scale)
//!   LANG <ru|en>         language tag for untagged SPEAK text (default ru)
//!   SPEAK <base64-utf8>  synthesize + play, interrupting current speech
//!   PAUSE / RESUME / STOP / SEEK <-30..30> / SPEED <50..300>
//!   EOF on stdin (parent exits) -> this process exits.
//!
//! Stdout handshake (one ASCII line per event, no text/credentials):
//!   READY engine=tera revision=<hex> voices=<list> sample_rate=44100 state=<ready|not-installed|error>
//!   STARTED id=<n> / DONE id=<n> / FAILED id=<n> reason=<token>
//!   REJECTED reason=<token>
//!
//! Every STARTED gets exactly one terminal event: DONE (natural finish or
//! interruption) or FAILED (synthesis cannot run). Only the worker thread
//! writes stdout, so protocol lines never interleave.
//!
//! Cancellation generations: synthesis runs on a DEDICATED worker thread so
//! the protocol loop stays responsive while a chunk is synthesizing — STOP
//! and a newer SPEAK are honoured immediately, playback stops at once (never
//! waiting for CPU synthesis), and the active utterance id is the
//! generation: inference results tagged with a superseded id are dropped and
//! can never reach playback.

mod chunk;
mod indexer;
mod manifest;
mod npy;
mod num2words;
#[cfg(windows)]
mod playback;
#[cfg(not(windows))]
#[path = "playback_macos.rs"]
mod playback;
mod protocol;
mod rng;
mod tera;
mod textnorm;

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};

use protocol::{Cmd, Event, RejectReason};

/// `%APPDATA%\suflyor\tts` — shared TTS root (Piper voices live in their own
/// subdirectories; Tera owns `teratts-v2-<revision>`).
fn tts_root() -> Option<PathBuf> {
    if let Some(a) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(a).join("suflyor").join("tts"));
    }
    if let Some(h) = std::env::var_os("HOME") {
        return Some(
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join("suflyor")
                .join("tts"),
        );
    }
    None
}

fn emit(out: &mut impl Write, event: &Event) {
    let _ = writeln!(out, "{}", event.to_line());
    let _ = out.flush();
}

/// Channel payload: a parsed command, the refusal of an unparsable line, a
/// synth result, or a playback-exit notification.
enum Message {
    Command(Cmd),
    Reject(RejectReason),
    Synth(SynthOutcome),
    PlaybackDone(u64),
    Shutdown,
}

/// One chunk of synthesis requested from the synth worker thread. The
/// utterance id doubles as the CANCELLATION GENERATION: a result whose id no
/// longer matches the active utterance is dropped without reaching playback.
#[derive(Debug, Clone, PartialEq)]
struct SynthJob {
    utterance: u64,
    chunk_index: usize,
    text: String,
    voice: String,
    lang: String,
    duration_scale: f32,
    seed: u64,
}

/// Result of synthesizing one chunk. `Err` carries a generic protocol reason
/// token only — never user text.
struct SynthOutcome {
    utterance: u64,
    audio: Result<Vec<Vec<f32>>, String>,
}

/// Audio sink for one utterance. Real builds wrap the WASAPI
/// [`playback::Playback`]; tests drive a fake to observe feed/stop
/// deterministically.
trait Player {
    fn feed(&mut self, samples: Vec<f32>);
    fn end_of_stream(&mut self);
    fn pause(&mut self);
    fn resume(&mut self);
    fn seek_seconds(&mut self, seconds: i32);
    fn set_speed(&mut self, speed: f32);
    fn stop(self);
}

/// Where [`SynthJob`]s go. Real builds hand them to the synth worker thread;
/// tests record them and inject results by hand.
trait SynthDispatch {
    fn dispatch(&mut self, job: SynthJob);
}

/// Voice preference when none is configured: the recommended Russian prompts
/// first (upstream README), then anything installed.
fn pick_default_voice(voices: &[String]) -> Option<String> {
    for preferred in ["ru_f1", "ru_m5", "ru_f2", "ru_m1"] {
        if voices.iter().any(|v| v == preferred) {
            return Some(preferred.to_string());
        }
    }
    voices.first().cloned()
}

/// First `:`-separated segment of an anyhow chain, reduced to protocol-safe
/// characters — the reason token. Error text (or anything resembling user
/// content) can never leak into a status line.
fn reason_token(err: &anyhow::Error, fallback: &str) -> String {
    let chain = format!("{err:#}");
    let head = chain.split([':', '\n']).next().unwrap_or("").trim();
    let token: String = head
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if token.is_empty() {
        fallback.to_string()
    } else {
        token
    }
}

/// Protocol state machine, decoupled from threads and WASAPI so STOP /
/// newer-SPEAK / stale-result handling is unit-testable without timing
/// sleeps.
///
/// Cancellation generations: the ACTIVE UTTERANCE ID is the generation. STOP
/// and a newer SPEAK invalidate it immediately — playback stops at once and
/// inference results arriving later under the old id are dropped, so stale
/// synthesis can never reach the speakers.
struct Controller<P: Player> {
    /// Installed voice styles (from the startup release scan; empty when the
    /// model is missing — SPEAK then fails via the synth worker's load).
    voices: Vec<String>,
    voice: Option<String>,
    lang: String,
    rate: i32,
    playback_speed_percent: i32,
    next_id: u64,
    active: Option<u64>,
    player: Option<P>,
    chunks_total: usize,
    chunks_settled: usize,
    playing_emitted: bool,
    /// Cancellation generation, shared with the synth worker thread.
    generation: Arc<AtomicU64>,
    make_player: Box<dyn FnMut(u32, u64) -> Result<P, String>>,
    dispatch: Box<dyn SynthDispatch>,
    emit: Box<dyn FnMut(&Event)>,
}

impl<P: Player> Controller<P> {
    fn emit_event(&mut self, event: Event) {
        (self.emit)(&event);
    }

    fn emit_failed(&mut self, id: u64, reason: &str) {
        self.emit_event(Event::Failed {
            id,
            reason: reason.to_string(),
        });
    }

    /// RATE → Tera `duration_scale`: higher rate means shorter durations.
    fn duration_scale(&self) -> f32 {
        let speed = 2.0_f32.powf(self.rate.clamp(-10, 10) as f32 / 10.0);
        1.0 / speed
    }

    /// End the active utterance NOW: playback stops immediately (synthesis
    /// runs on its own thread and its now-stale results are dropped by
    /// generation), and the replaced/stopped utterance receives its terminal
    /// DONE. Interruption ends an utterance normally — exactly one terminal
    /// event per STARTED.
    fn close_active(&mut self) {
        if let Some(player) = self.player.take() {
            player.stop();
        }
        self.chunks_total = 0;
        self.chunks_settled = 0;
        self.playing_emitted = false;
        let previous = self.active.take();
        self.generation.store(0, Ordering::Release);
        if let Some(id) = previous {
            self.emit_event(Event::Done { id });
        }
    }

    fn speak(&mut self, text: &str) {
        self.close_active();
        let id = self.next_id;
        self.next_id += 1;
        self.emit_event(Event::Started { id });

        let Some(voice) = self
            .voice
            .clone()
            .or_else(|| pick_default_voice(&self.voices))
        else {
            self.emit_failed(id, RejectReason::UnknownVoice.token());
            return;
        };
        // With a known voice list validate up front; when it is empty (model
        // missing) the synth worker's load failure reports the real reason.
        if !self.voices.is_empty() && !self.voices.iter().any(|v| v == &voice) {
            self.emit_failed(id, RejectReason::UnknownVoice.token());
            return;
        }
        let chunks = chunk::chunk_text(&chunk::sanitize(text));
        if chunks.is_empty() {
            self.emit_event(Event::Done { id });
            return;
        }
        match (self.make_player)(tera::SAMPLE_RATE, id) {
            Ok(mut player) => {
                player.set_speed(self.playback_speed_percent as f32 / 100.0);
                self.voice = Some(voice.clone());
                self.player = Some(player);
                self.active = Some(id);
                self.generation.store(id, Ordering::Release);
                self.chunks_total = chunks.len();
                self.chunks_settled = 0;
                self.playing_emitted = false;
                let scale = self.duration_scale();
                for (chunk_index, chunk_text) in chunks.into_iter().enumerate() {
                    self.dispatch.dispatch(SynthJob {
                        utterance: id,
                        chunk_index,
                        text: chunk_text,
                        voice: voice.clone(),
                        lang: self.lang.clone(),
                        duration_scale: scale,
                        seed: tera::SEED.wrapping_add(id),
                    });
                }
            }
            Err(err) => {
                eprintln!("[suflyor-teratts] playback start failed: {err}");
                self.emit_failed(id, "playback");
            }
        }
    }

    /// A synth result arrived. Results for a superseded generation (utterance
    /// replaced or stopped mid-synthesis) are dropped silently — their audio
    /// never reaches playback.
    fn on_synth_result(&mut self, outcome: SynthOutcome) {
        let Some(active) = self.active else {
            return;
        };
        if outcome.utterance != active {
            return;
        }
        self.chunks_settled += 1;
        match outcome.audio {
            Ok(audio) => {
                let has_audio = audio.iter().any(|samples| !samples.is_empty());
                if has_audio && !self.playing_emitted {
                    self.playing_emitted = true;
                    self.emit_event(Event::Playing { id: active });
                }
                if let Some(player) = self.player.as_mut() {
                    for samples in audio {
                        player.feed(samples);
                    }
                    if self.chunks_settled == self.chunks_total {
                        player.end_of_stream();
                    }
                }
            }
            Err(reason) => {
                eprintln!("[suflyor-teratts] synth failed ({reason})");
                if let Some(player) = self.player.take() {
                    player.stop();
                }
                self.active = None;
                self.chunks_total = 0;
                self.chunks_settled = 0;
                self.playing_emitted = false;
                self.generation.store(0, Ordering::Release);
                self.emit_event(Event::Failed { id: active, reason });
            }
        }
    }

    /// The render loop exited (EOS drained, device error, or our own stop).
    /// Only the ACTIVE utterance's notification ends it — a late notification
    /// from a stopped/replaced player is dropped.
    fn on_playback_done(&mut self, id: u64) {
        if self.active != Some(id) {
            return;
        }
        if let Some(player) = self.player.take() {
            player.stop();
        }
        self.active = None;
        self.chunks_total = 0;
        self.chunks_settled = 0;
        self.playing_emitted = false;
        self.generation.store(0, Ordering::Release);
        self.emit_event(Event::Done { id });
    }

    fn on_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Speak(text) => self.speak(&text),
            Cmd::Pause => {
                if let Some(player) = self.player.as_mut() {
                    player.pause();
                }
            }
            Cmd::Resume => {
                if let Some(player) = self.player.as_mut() {
                    player.resume();
                }
            }
            Cmd::Seek(seconds) => {
                if let Some(player) = self.player.as_mut() {
                    player.seek_seconds(seconds);
                }
            }
            Cmd::SetPlaybackSpeed(percent) => {
                self.playback_speed_percent = percent.clamp(50, 300);
                if let Some(player) = self.player.as_mut() {
                    player.set_speed(self.playback_speed_percent as f32 / 100.0);
                }
            }
            Cmd::Stop => self.close_active(),
            Cmd::SetRate(rate) => self.rate = rate.clamp(-10, 10),
            Cmd::SetVoice(id) => {
                if !self.voices.is_empty() && self.voices.iter().any(|v| v == &id) {
                    self.close_active();
                    self.voice = Some(id);
                } else {
                    self.emit_event(Event::Rejected {
                        reason: RejectReason::UnknownVoice,
                    });
                }
            }
            Cmd::SetLang(lang) => self.lang = lang,
        }
    }
}

/// Real WASAPI-backed player.
struct RealPlayer(playback::Playback);

impl Player for RealPlayer {
    fn feed(&mut self, samples: Vec<f32>) {
        self.0.feed(samples);
    }
    fn end_of_stream(&mut self) {
        self.0.end_of_stream();
    }
    fn pause(&mut self) {
        self.0.pause();
    }
    fn resume(&mut self) {
        self.0.resume();
    }
    fn seek_seconds(&mut self, seconds: i32) {
        self.0.seek_seconds(seconds);
    }
    fn set_speed(&mut self, speed: f32) {
        self.0.set_speed(speed);
    }
    fn stop(self) {
        self.0.stop();
    }
}

/// Hands jobs to the synth worker thread.
struct ThreadDispatch {
    jobs: mpsc::Sender<SynthJob>,
}

impl SynthDispatch for ThreadDispatch {
    fn dispatch(&mut self, job: SynthJob) {
        let _ = self.jobs.send(job);
    }
}

/// Used only if the synth thread cannot be spawned: fails every job
/// synchronously so the "every STARTED gets exactly one terminal event"
/// protocol invariant still holds.
struct FailingDispatch {
    events: mpsc::Sender<Message>,
}

impl SynthDispatch for FailingDispatch {
    fn dispatch(&mut self, job: SynthJob) {
        let _ = self.events.send(Message::Synth(SynthOutcome {
            utterance: job.utterance,
            audio: Err("load".to_string()),
        }));
    }
}

/// Dedicated synthesis worker: owns the lazily-loaded engine so CPU-heavy
/// inference never blocks the protocol loop — STOP and a newer SPEAK stay
/// observable while one chunk is synthesizing. Jobs for superseded
/// generations are skipped before spending CPU.
fn synth_worker(
    root: PathBuf,
    generation: Arc<AtomicU64>,
    jobs: mpsc::Receiver<SynthJob>,
    events: mpsc::Sender<Message>,
) {
    let mut engine = match tera::TeraEngine::load(&root) {
        Ok(loaded) => Some(loaded),
        Err(err) => {
            eprintln!(
                "[suflyor-teratts] background warm-up failed ({})",
                reason_token(&err, "load")
            );
            None
        }
    };
    while let Ok(job) = jobs.recv() {
        if generation.load(Ordering::Acquire) != job.utterance {
            continue;
        }
        if engine.is_none() {
            match tera::TeraEngine::load(&root) {
                Ok(loaded) => engine = Some(loaded),
                Err(err) => {
                    let reason = reason_token(&err, "load");
                    let _ = events.send(Message::Synth(SynthOutcome {
                        utterance: job.utterance,
                        audio: Err(reason),
                    }));
                    continue;
                }
            }
        }
        let audio = match engine.as_mut() {
            Some(loaded) => loaded
                .synthesize(
                    &job.text,
                    &job.voice,
                    &job.lang,
                    job.duration_scale,
                    job.seed,
                )
                .map(|output| output.chunks)
                .map_err(|err| reason_token(&err, "synth")),
            None => Err("load".to_string()),
        };
        let _ = events.send(Message::Synth(SynthOutcome {
            utterance: job.utterance,
            audio,
        }));
    }
}

fn worker(mut controller: Controller<RealPlayer>, rx: mpsc::Receiver<Message>) {
    while let Ok(message) = rx.recv() {
        match message {
            Message::Command(cmd) => controller.on_cmd(cmd),
            Message::Reject(reason) => controller.emit_event(Event::Rejected { reason }),
            Message::Synth(outcome) => controller.on_synth_result(outcome),
            Message::PlaybackDone(id) => controller.on_playback_done(id),
            Message::Shutdown => break,
        }
    }
    // stdin closed (the app exited): stop speech immediately.
    controller.close_active();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let status_mode = args.get(1).map(String::as_str) == Some("status");

    let root = match tts_root() {
        Some(r) => r,
        None => {
            eprintln!("[suflyor-teratts] APPDATA not set");
            if status_mode {
                println!("READY engine=tera revision= voices= sample_rate=44100 state=error");
            }
            std::process::exit(if status_mode { 3 } else { 1 });
        }
    };

    // The pinned manifest is compiled in; failure here would be a build
    // defect, and the protocol answers with state=error instead of crashing.
    let (revision, voices, state) = match manifest::Manifest::pinned() {
        Ok(manifest) => {
            let release = manifest.release_dir(&root);
            match manifest::check_installed(&manifest, &release) {
                Ok(()) => (
                    manifest.revision.clone(),
                    manifest::installed_voices(&release),
                    "ready",
                ),
                Err(_) => (manifest.revision.clone(), Vec::new(), "not-installed"),
            }
        }
        Err(_) => (String::new(), Vec::new(), "error"),
    };

    let mut out = std::io::stdout();
    emit(
        &mut out,
        &Event::Ready {
            revision,
            voices: voices.clone(),
            sample_rate: tera::SAMPLE_RATE,
            state: state.to_string(),
        },
    );

    if status_mode {
        std::process::exit(if state == "ready" { 0 } else { 2 });
    }

    let generation = Arc::new(AtomicU64::new(0));
    let (events_tx, events_rx) = mpsc::channel::<Message>();
    let (jobs_tx, jobs_rx) = mpsc::channel::<SynthJob>();

    let dispatch: Box<dyn SynthDispatch> = match std::thread::Builder::new()
        .name("teratts-synth".into())
        .spawn({
            let root = root.clone();
            let generation = generation.clone();
            let events = events_tx.clone();
            move || synth_worker(root, generation, jobs_rx, events)
        }) {
        Ok(_) => Box::new(ThreadDispatch { jobs: jobs_tx }),
        Err(err) => {
            eprintln!("[suflyor-teratts] synth worker spawn failed: {err}");
            Box::new(FailingDispatch {
                events: events_tx.clone(),
            })
        }
    };

    let player_events = events_tx.clone();
    let make_player = move |sample_rate: u32, utterance: u64| -> Result<RealPlayer, String> {
        let notify = player_events.clone();
        playback::Playback::start(
            sample_rate,
            Some(Box::new(move || {
                let _ = notify.send(Message::PlaybackDone(utterance));
            })),
        )
        .map(RealPlayer)
        .map_err(|e| format!("{e:#}"))
    };

    let controller = Controller {
        voices,
        voice: None,
        lang: "ru".to_string(),
        rate: 0,
        playback_speed_percent: 100,
        next_id: 1,
        active: None,
        player: None,
        chunks_total: 0,
        chunks_settled: 0,
        playing_emitted: false,
        generation,
        make_player: Box::new(make_player),
        dispatch,
        emit: Box::new(|event: &Event| {
            let mut out = std::io::stdout();
            emit(&mut out, event);
        }),
    };

    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            let message = match protocol::parse_cmd(&line) {
                Ok(Some(cmd)) => Message::Command(cmd),
                Ok(None) => continue,
                Err(reason) => Message::Reject(reason),
            };
            if events_tx.send(message).is_err() {
                return;
            }
        }
        let _ = events_tx.send(Message::Shutdown);
    });
    worker(controller, events_rx);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Deterministic fake player: every interaction lands on a shared log.
    struct FakePlayer {
        utterance: u64,
        log: Rc<RefCell<Vec<String>>>,
    }

    impl Player for FakePlayer {
        fn feed(&mut self, samples: Vec<f32>) {
            self.log
                .borrow_mut()
                .push(format!("feed:{}:{}", self.utterance, samples.len()));
        }
        fn end_of_stream(&mut self) {
            self.log
                .borrow_mut()
                .push(format!("eos:{}", self.utterance));
        }
        fn pause(&mut self) {
            self.log
                .borrow_mut()
                .push(format!("pause:{}", self.utterance));
        }
        fn resume(&mut self) {
            self.log
                .borrow_mut()
                .push(format!("resume:{}", self.utterance));
        }
        fn seek_seconds(&mut self, seconds: i32) {
            self.log
                .borrow_mut()
                .push(format!("seek:{}:{seconds}", self.utterance));
        }
        fn set_speed(&mut self, speed: f32) {
            self.log
                .borrow_mut()
                .push(format!("speed:{}:{speed:.2}", self.utterance));
        }
        fn stop(self) {
            self.log
                .borrow_mut()
                .push(format!("stop:{}", self.utterance));
        }
    }

    /// Records dispatched jobs; tests inject the results by hand, in any
    /// order — no threads, no timing.
    struct RecordingDispatch {
        jobs: Rc<RefCell<Vec<SynthJob>>>,
    }

    impl SynthDispatch for RecordingDispatch {
        fn dispatch(&mut self, job: SynthJob) {
            self.jobs.borrow_mut().push(job);
        }
    }

    struct Harness {
        controller: Controller<FakePlayer>,
        events: Rc<RefCell<Vec<Event>>>,
        jobs: Rc<RefCell<Vec<SynthJob>>>,
        player_log: Rc<RefCell<Vec<String>>>,
        generation: Arc<AtomicU64>,
    }

    fn harness_with_voices(voices: Vec<String>) -> Harness {
        let events: Rc<RefCell<Vec<Event>>> = Rc::new(RefCell::new(Vec::new()));
        let jobs: Rc<RefCell<Vec<SynthJob>>> = Rc::new(RefCell::new(Vec::new()));
        let player_log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let generation = Arc::new(AtomicU64::new(0));

        let log_for_player = player_log.clone();
        let make_player = move |_rate: u32, utterance: u64| -> Result<FakePlayer, String> {
            Ok(FakePlayer {
                utterance,
                log: log_for_player.clone(),
            })
        };
        let events_out = events.clone();
        Harness {
            controller: Controller {
                voices,
                voice: None,
                lang: "ru".to_string(),
                rate: 0,
                playback_speed_percent: 100,
                next_id: 1,
                active: None,
                player: None,
                chunks_total: 0,
                chunks_settled: 0,
                playing_emitted: false,
                generation: generation.clone(),
                make_player: Box::new(make_player),
                dispatch: Box::new(RecordingDispatch { jobs: jobs.clone() }),
                emit: Box::new(move |event: &Event| events_out.borrow_mut().push(event.clone())),
            },
            events,
            jobs,
            player_log,
            generation,
        }
    }

    fn harness() -> Harness {
        harness_with_voices(vec!["ru_f1".into(), "ru_m5".into()])
    }

    fn take_events(h: &Harness) -> Vec<Event> {
        std::mem::take(&mut *h.events.borrow_mut())
    }

    fn ok_audio(utterance: u64) -> SynthOutcome {
        SynthOutcome {
            utterance,
            audio: Ok(vec![vec![0.5; 4]]),
        }
    }

    /// Long enough to chunk into several synthesis jobs.
    fn long_text() -> String {
        "Длинное предложение для теста. ".repeat(30)
    }

    #[test]
    fn stop_during_synthesis_stops_immediately_and_discards_stale_audio() {
        let mut h = harness();
        h.controller.on_cmd(Cmd::Speak(long_text()));
        let total = h.jobs.borrow().len();
        assert!(total >= 2, "expected multiple chunks, got {total}");
        assert_eq!(h.generation.load(Ordering::Acquire), 1);

        // STOP while every chunk is still "synthesizing" (no results yet).
        h.controller.on_cmd(Cmd::Stop);
        assert_eq!(
            take_events(&h),
            vec![Event::Started { id: 1 }, Event::Done { id: 1 }]
        );
        assert!(h.player_log.borrow().iter().any(|e| e == "stop:1"));
        assert_eq!(h.generation.load(Ordering::Acquire), 0);

        // All stale results arrive late: nothing plays, nothing is emitted.
        for _ in 0..total {
            h.controller.on_synth_result(ok_audio(1));
        }
        assert!(take_events(&h).is_empty());
        assert!(!h.player_log.borrow().iter().any(|e| e.starts_with("feed:")));
    }

    #[test]
    fn newer_speak_supersedes_in_flight_synthesis() {
        let mut h = harness();
        h.controller.on_cmd(Cmd::Speak(long_text()));
        let stale_jobs = h.jobs.borrow().clone();
        assert!(!stale_jobs.is_empty());

        // A newer SPEAK mid-synthesis interrupts the old utterance.
        h.controller.on_cmd(Cmd::Speak("Короткая замена.".into()));
        assert_eq!(
            take_events(&h),
            vec![
                Event::Started { id: 1 },
                Event::Done { id: 1 },
                Event::Started { id: 2 }
            ]
        );
        assert_eq!(h.generation.load(Ordering::Acquire), 2);

        // Old-generation results are dropped without playing.
        for job in &stale_jobs {
            h.controller.on_synth_result(ok_audio(job.utterance));
        }
        assert!(take_events(&h).is_empty());
        assert!(!h
            .player_log
            .borrow()
            .iter()
            .any(|e| e.starts_with("feed:1")));

        // The new generation plays and finishes normally.
        h.controller.on_synth_result(ok_audio(2));
        assert_eq!(take_events(&h), vec![Event::Playing { id: 2 }]);
        assert!(h.player_log.borrow().iter().any(|e| e == "feed:2:4"));
        assert!(h.player_log.borrow().iter().any(|e| e == "eos:2"));
        h.controller.on_playback_done(2);
        assert_eq!(take_events(&h), vec![Event::Done { id: 2 }]);
    }

    #[test]
    fn playing_is_emitted_once_when_first_audio_reaches_the_player() {
        let mut h = harness();
        h.controller.on_cmd(Cmd::Speak(long_text()));
        take_events(&h);

        h.controller.on_synth_result(ok_audio(1));
        h.controller.on_synth_result(ok_audio(1));

        assert_eq!(take_events(&h), vec![Event::Playing { id: 1 }]);
    }

    #[test]
    fn every_started_gets_exactly_one_terminal_event() {
        let mut h = harness();
        // id 1 superseded, id 2 stopped, id 3 synth-fails, id 4 finishes.
        h.controller.on_cmd(Cmd::Speak(long_text()));
        h.controller.on_cmd(Cmd::Speak(long_text()));
        h.controller.on_cmd(Cmd::Stop);
        h.controller.on_cmd(Cmd::Speak("Текст.".into()));
        h.controller.on_synth_result(SynthOutcome {
            utterance: 3,
            audio: Err("synth".into()),
        });
        h.controller.on_cmd(Cmd::Speak("Финал.".into()));
        h.controller.on_synth_result(ok_audio(4));
        h.controller.on_playback_done(4);

        let events = h.events.borrow();
        let mut started: Vec<u64> = events
            .iter()
            .filter_map(|e| match e {
                Event::Started { id } => Some(*id),
                _ => None,
            })
            .collect();
        started.sort_unstable();
        assert_eq!(started, vec![1, 2, 3, 4]);
        for id in [1u64, 2, 3, 4] {
            let terminals = events
                .iter()
                .filter(|e| matches!(e, Event::Done { id: x } | Event::Failed { id: x, .. } if *x == id))
                .count();
            assert_eq!(terminals, 1, "utterance {id} terminal count");
        }
    }

    #[test]
    fn synth_failure_emits_generic_failed_and_stops_playback() {
        let mut h = harness();
        h.controller.on_cmd(Cmd::Speak("Текст.".into()));
        h.controller.on_synth_result(SynthOutcome {
            utterance: 1,
            audio: Err("synth".into()),
        });
        assert_eq!(
            take_events(&h),
            vec![
                Event::Started { id: 1 },
                Event::Failed {
                    id: 1,
                    reason: "synth".into()
                }
            ]
        );
        assert!(h.player_log.borrow().iter().any(|e| e == "stop:1"));
        // A late duplicate result for the failed utterance is dropped.
        h.controller.on_synth_result(ok_audio(1));
        assert!(take_events(&h).is_empty());
    }

    #[test]
    fn unknown_voice_fails_without_dispatching_synthesis() {
        let mut h = harness();
        h.controller.voice = Some("ghost".into());
        h.controller.on_cmd(Cmd::Speak("Текст.".into()));
        assert_eq!(
            take_events(&h),
            vec![
                Event::Started { id: 1 },
                Event::Failed {
                    id: 1,
                    reason: "unknown-voice".into()
                }
            ]
        );
        assert!(h.jobs.borrow().is_empty());
    }

    #[test]
    fn empty_text_done_without_player_or_dispatch() {
        let mut h = harness();
        h.controller.on_cmd(Cmd::Speak("   ".into()));
        assert_eq!(
            take_events(&h),
            vec![Event::Started { id: 1 }, Event::Done { id: 1 }]
        );
        assert!(h.jobs.borrow().is_empty());
        assert!(h.player_log.borrow().is_empty());
    }

    #[test]
    fn voice_command_validates_against_installed_styles() {
        let mut h = harness();
        h.controller.on_cmd(Cmd::SetVoice("ru_m5".into()));
        assert!(take_events(&h).is_empty());
        assert_eq!(h.controller.voice.as_deref(), Some("ru_m5"));
        h.controller.on_cmd(Cmd::SetVoice("ghost".into()));
        assert_eq!(
            take_events(&h),
            vec![Event::Rejected {
                reason: RejectReason::UnknownVoice
            }]
        );
        assert_eq!(h.controller.voice.as_deref(), Some("ru_m5"));
    }

    #[test]
    fn pause_resume_forward_to_the_active_player() {
        let mut h = harness();
        h.controller.on_cmd(Cmd::Speak("Текст.".into()));
        h.controller.on_cmd(Cmd::Pause);
        h.controller.on_cmd(Cmd::Resume);
        let log = h.player_log.borrow();
        assert!(log.iter().any(|e| e == "pause:1"));
        assert!(log.iter().any(|e| e == "resume:1"));
    }

    #[test]
    fn seek_and_speed_apply_to_the_active_and_next_generation() {
        let mut h = harness();
        h.controller.on_cmd(Cmd::Seek(-10));
        h.controller.on_cmd(Cmd::SetPlaybackSpeed(150));
        assert!(h.player_log.borrow().is_empty());

        h.controller.on_cmd(Cmd::Speak("Text.".into()));
        assert!(h.player_log.borrow().iter().any(|e| e == "speed:1:1.50"));
        h.controller.on_cmd(Cmd::Seek(-10));
        h.controller.on_cmd(Cmd::SetPlaybackSpeed(150));
        let log = h.player_log.borrow();
        assert!(log.iter().any(|e| e == "seek:1:-10"));
        assert!(log.iter().any(|e| e == "speed:1:1.50"));
    }

    #[test]
    fn stale_playback_done_is_ignored() {
        let mut h = harness();
        h.controller.on_cmd(Cmd::Speak("Текст.".into()));
        h.controller.on_cmd(Cmd::Stop);
        take_events(&h);
        // Late notification from the stopped player: no second terminal event.
        h.controller.on_playback_done(1);
        assert!(take_events(&h).is_empty());
    }

    #[test]
    fn generation_tracks_the_active_utterance() {
        let mut h = harness();
        assert_eq!(h.generation.load(Ordering::Acquire), 0);
        h.controller.on_cmd(Cmd::Speak("Раз.".into()));
        assert_eq!(h.generation.load(Ordering::Acquire), 1);
        h.controller.on_cmd(Cmd::Speak("Два.".into()));
        assert_eq!(h.generation.load(Ordering::Acquire), 2);
        h.controller.on_cmd(Cmd::Stop);
        assert_eq!(h.generation.load(Ordering::Acquire), 0);
    }

    #[test]
    fn lang_and_rate_flow_into_jobs() {
        let mut h = harness();
        h.controller.on_cmd(Cmd::SetLang("en".into()));
        h.controller.on_cmd(Cmd::SetRate(42));
        assert_eq!(h.controller.rate, 10);
        h.controller.on_cmd(Cmd::Speak("Text.".into()));
        let jobs = h.jobs.borrow();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].lang, "en");
        // rate +10 → speed 2× → duration_scale 0.5
        assert!((jobs[0].duration_scale - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn reason_tokens_are_protocol_safe() {
        assert_eq!(
            reason_token(&anyhow::anyhow!("not-installed: marker missing"), "x"),
            "not-installed"
        );
        assert_eq!(
            reason_token(&anyhow::anyhow!("synth: sampler failed: ORT error"), "x"),
            "synth"
        );
        // No colon: the whole head is filtered to protocol-safe ASCII.
        assert_eq!(reason_token(&anyhow::anyhow!("SYNTH bad"), "x"), "synthbad");
        // Nothing usable falls back to the generic token.
        assert_eq!(reason_token(&anyhow::anyhow!("ошибка"), "x"), "x");
        assert_eq!(reason_token(&anyhow::anyhow!(""), "fallback"), "fallback");
    }
}
