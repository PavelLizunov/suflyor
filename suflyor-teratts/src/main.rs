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
//!   PAUSE / RESUME / STOP
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

mod chunk;
mod indexer;
mod manifest;
mod npy;
mod num2words;
mod playback;
mod protocol;
mod rng;
mod tera;
mod textnorm;

use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc;

use protocol::{Cmd, Event, RejectReason};

/// `%APPDATA%\suflyor\tts` — shared TTS root (Piper voices live in their own
/// subdirectories; Tera owns `teratts-v2-<revision>`).
fn tts_root() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("suflyor").join("tts"))
}

fn emit(out: &mut impl Write, event: &Event) {
    let _ = writeln!(out, "{}", event.to_line());
    let _ = out.flush();
}

/// Channel payload: a parsed command, or the refusal of an unparsable line.
enum Message {
    Command(Cmd),
    Reject(RejectReason),
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

/// First `:`-separated segment of an anyhow chain — the reason token.
fn reason_token(err: &anyhow::Error, fallback: &str) -> String {
    let chain = format!("{err:#}");
    let token = chain.split([':', '\n']).next().unwrap_or(fallback).trim();
    if token.is_empty() {
        fallback.to_string()
    } else {
        token.to_string()
    }
}

fn emit_failed(out: &mut impl Write, id: u64, reason: String) {
    emit(out, &Event::Failed { id, reason });
}

struct Worker {
    root: PathBuf,
    engine: Option<tera::TeraEngine>,
    voices: Vec<String>,
    voice: Option<String>,
    lang: String,
    rate: i32,
    next_id: u64,
    active_id: Option<u64>,
    playback: Option<playback::Playback>,
    pending: VecDeque<String>,
}

impl Worker {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            engine: None,
            voices: Vec::new(),
            voice: None,
            lang: "ru".to_string(),
            rate: 0,
            next_id: 1,
            active_id: None,
            playback: None,
            pending: VecDeque::new(),
        }
    }

    /// RATE → Tera `duration_scale`: higher rate means shorter durations.
    fn duration_scale(&self) -> f32 {
        let speed = 2.0_f32.powf(self.rate.clamp(-10, 10) as f32 / 10.0);
        1.0 / speed
    }

    /// Stop any active utterance. Interruption ends the utterance normally,
    /// so the replaced generation still receives its DONE.
    fn close_active(&mut self, out: &mut impl Write) {
        self.pending.clear();
        if let Some(pb) = self.playback.take() {
            pb.stop();
        }
        if let Some(id) = self.active_id.take() {
            emit(out, &Event::Done { id });
        }
    }

    /// Load the engine on first use. Leaves `self.engine`/`self.voices`
    /// populated on success.
    fn ensure_engine(&mut self) -> Result<(), String> {
        if self.engine.is_none() {
            match tera::TeraEngine::load(&self.root) {
                Ok(engine) => {
                    self.voices = engine.voices();
                    self.engine = Some(engine);
                }
                Err(err) => return Err(reason_token(&err, "load")),
            }
        }
        if self.engine.is_some() {
            Ok(())
        } else {
            Err("load".to_string())
        }
    }

    fn handle_speak(&mut self, text: &str, out: &mut impl Write) {
        self.close_active(out);
        let id = self.next_id;
        self.next_id += 1;
        emit(out, &Event::Started { id });

        if let Err(reason) = self.ensure_engine() {
            emit_failed(out, id, reason);
            return;
        }
        let Some(voice) = self
            .voice
            .clone()
            .or_else(|| pick_default_voice(&self.voices))
        else {
            emit_failed(out, id, RejectReason::UnknownVoice.token().to_string());
            return;
        };
        if !self.voices.iter().any(|v| v == &voice) {
            emit_failed(out, id, RejectReason::UnknownVoice.token().to_string());
            return;
        }
        let Some(sample_rate) = self.engine.as_ref().map(|e| e.sample_rate()) else {
            emit_failed(out, id, "load".to_string());
            return;
        };
        self.voice = Some(voice);

        let clean = chunk::sanitize(text);
        let chunks = chunk::chunk_text(&clean);
        if chunks.is_empty() {
            emit(out, &Event::Done { id });
            return;
        }
        match playback::Playback::start(sample_rate) {
            Ok(pb) => {
                self.active_id = Some(id);
                self.playback = Some(pb);
                self.pending = VecDeque::from(chunks);
            }
            Err(err) => {
                eprintln!("[suflyor-teratts] playback start failed: {err:#}");
                emit_failed(out, id, "playback".to_string());
            }
        }
    }

    /// Feed one more chunk to synthesis/playback, or finalize the utterance.
    fn idle_tick(&mut self, out: &mut impl Write) {
        if self.pending.is_empty() {
            if let Some(pb) = &self.playback {
                if pb.is_finished() {
                    if let Some(pb) = self.playback.take() {
                        pb.stop();
                    }
                    if let Some(id) = self.active_id.take() {
                        emit(out, &Event::Done { id });
                    }
                    return;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            return;
        }

        let Some(id) = self.active_id else {
            self.pending.clear();
            return;
        };
        let Some(chunk) = self.pending.pop_front() else {
            return;
        };
        let voice = self.voice.clone();
        let lang = self.lang.clone();
        let scale = self.duration_scale();
        let seed = tera::SEED.wrapping_add(id);
        let synth = self.engine.as_mut().and_then(|engine| {
            voice
                .as_ref()
                .map(|v| engine.synthesize(&chunk, v, &lang, scale, seed))
        });
        match synth {
            Some(Ok(output)) => {
                if let Some(pb) = &self.playback {
                    let last = self.pending.is_empty();
                    for c in output.chunks {
                        pb.feed(c);
                    }
                    if last {
                        pb.end_of_stream();
                    }
                }
            }
            Some(Err(err)) => {
                let reason = reason_token(&err, "synth");
                eprintln!("[suflyor-teratts] synth failed ({reason})");
                self.pending.clear();
                if let Some(pb) = self.playback.take() {
                    pb.stop();
                }
                self.active_id = None;
                emit(out, &Event::Failed { id, reason });
            }
            None => {
                self.pending.clear();
                if let Some(pb) = self.playback.take() {
                    pb.stop();
                }
                self.active_id = None;
                emit(
                    out,
                    &Event::Failed {
                        id,
                        reason: "load".to_string(),
                    },
                );
            }
        }
    }
}

fn worker(rx: mpsc::Receiver<Message>, root: PathBuf) {
    let mut out = std::io::stdout();
    let mut w = Worker::new(root);

    loop {
        let msg = if w.pending.is_empty() && w.playback.is_none() {
            match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => break,
            }
        } else {
            match rx.try_recv() {
                Ok(m) => Some(m),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        };

        match msg {
            Some(Message::Command(Cmd::Speak(text))) => w.handle_speak(&text, &mut out),
            Some(Message::Command(Cmd::Pause)) => {
                if let Some(pb) = &w.playback {
                    pb.pause();
                }
            }
            Some(Message::Command(Cmd::Resume)) => {
                if let Some(pb) = &w.playback {
                    pb.resume();
                }
            }
            Some(Message::Command(Cmd::Stop)) => w.close_active(&mut out),
            Some(Message::Command(Cmd::SetRate(r))) => w.rate = r.clamp(-10, 10),
            Some(Message::Command(Cmd::SetVoice(id))) => {
                if w.engine.is_none() {
                    let _ = w.ensure_engine();
                }
                if w.voices.iter().any(|v| v == &id) {
                    w.close_active(&mut out);
                    w.voice = Some(id);
                } else {
                    emit(
                        &mut out,
                        &Event::Rejected {
                            reason: RejectReason::UnknownVoice,
                        },
                    );
                }
            }
            Some(Message::Command(Cmd::SetLang(lang))) => w.lang = lang,
            Some(Message::Reject(reason)) => emit(&mut out, &Event::Rejected { reason }),
            None => w.idle_tick(&mut out),
        }
    }

    // stdin closed (the app exited): stop speech immediately.
    w.close_active(&mut out);
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
            voices,
            sample_rate: tera::SAMPLE_RATE,
            state: state.to_string(),
        },
    );

    if status_mode {
        std::process::exit(if state == "ready" { 0 } else { 2 });
    }

    let (tx, rx) = mpsc::channel::<Message>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            let message = match protocol::parse_cmd(&line) {
                Ok(Some(cmd)) => Message::Command(cmd),
                Ok(None) => continue,
                Err(reason) => Message::Reject(reason),
            };
            if tx.send(message).is_err() {
                break;
            }
        }
    });
    worker(rx, root);
}
