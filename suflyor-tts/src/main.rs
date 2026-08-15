//! suflyor-tts — neural read-aloud sidecar.
//!
//! Reads one command per line on stdin and synthesizes + plays Russian speech
//! via sherpa-onnx + WASAPI. Lives in its own process so its onnxruntime never
//! shares a binary with the main app's `ort`/GigaAM STT runtime (the two collide
//! when static-linked together → native crash).
//!
//! Protocol (stdin, one per line):
//!   VOICE <dir>          select voice by model-dir name (loads on next SPEAK)
//!   RATE <-10..10>       set read rate
//!   SPEAK <base64-utf8>  synthesize + play, interrupting any current speech
//!   PAUSE / RESUME / STOP / SEEK <-30..30> / SPEED <50..300>
//! stdout emits READY, STARTED id=<n>, and a matching DONE/FAILED event so the
//! host keeps controls visible until the render device has actually drained.
//! EOF on stdin (parent exits) → this process exits.

mod diar;
mod engine;
mod playback;

use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::sync::mpsc;

use base64::Engine as _;

use engine::NeuralEngine;
use playback::Playback;

enum Cmd {
    Speak(String),
    Pause,
    Resume,
    Stop,
    Seek(i32),
    SetPlaybackSpeed(i32),
    SetRate(i32),
    SetVoice(String),
}

enum Message {
    Command(Cmd),
    PlaybackDone(u64),
    Shutdown,
}

#[derive(Clone, Copy)]
struct PlaybackSpeed(i32);

impl Default for PlaybackSpeed {
    fn default() -> Self {
        Self(100)
    }
}

impl PlaybackSpeed {
    fn set(&mut self, percent: i32) -> f32 {
        self.0 = percent.clamp(50, 300);
        self.factor()
    }

    fn factor(self) -> f32 {
        self.0 as f32 / 100.0
    }
}

fn take_finished_playback<P>(current: &mut Option<(u64, P)>, id: u64) -> Option<P> {
    if current.as_ref().map(|(active, _)| *active) != Some(id) {
        return None;
    }
    current.take().map(|(_, playback)| playback)
}

fn parse_cmd(line: &str) -> Option<Cmd> {
    let line = line.trim_end_matches(['\r', '\n']);
    match line {
        "PAUSE" => return Some(Cmd::Pause),
        "RESUME" => return Some(Cmd::Resume),
        "STOP" => return Some(Cmd::Stop),
        _ => {}
    }
    if let Some(rest) = line.strip_prefix("RATE ") {
        return rest.trim().parse::<i32>().ok().map(Cmd::SetRate);
    }
    if let Some(rest) = line.strip_prefix("SEEK ") {
        return rest
            .trim()
            .parse::<i32>()
            .ok()
            .filter(|seconds| (-30..=30).contains(seconds))
            .map(Cmd::Seek);
    }
    if let Some(rest) = line.strip_prefix("SPEED ") {
        return rest
            .trim()
            .parse::<i32>()
            .ok()
            .filter(|percent| (50..=300).contains(percent))
            .map(Cmd::SetPlaybackSpeed);
    }
    if let Some(rest) = line.strip_prefix("VOICE ") {
        return Some(Cmd::SetVoice(rest.trim().to_string()));
    }
    if let Some(rest) = line.strip_prefix("SPEAK ") {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(rest.trim())
            .ok()?;
        let text = String::from_utf8(bytes).ok()?;
        return Some(Cmd::Speak(text));
    }
    None
}

fn main() {
    // Subcommand dispatch: `diarize <wav> …` runs a one-shot speaker diarization,
    // prints JSON, and exits (D1). No args → the read-aloud stdin loop below,
    // byte-identical to before. One exe, two jobs, ALWAYS separate OS processes —
    // a live read-aloud and a diarize batch never share an address space.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("diarize") {
        std::process::exit(diar::run_cli(&args[2..]));
    }

    // stdin → Cmd channel. Dropping `tx` on EOF makes the worker's recv() return
    // Err, which exits the process.
    let (tx, rx) = mpsc::channel::<Message>();
    let worker_tx = tx.clone();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if let Some(cmd) = parse_cmd(&line) {
                if tx.send(Message::Command(cmd)).is_err() {
                    break;
                }
            }
        }
        let _ = tx.send(Message::Shutdown);
    });
    worker(rx, worker_tx);
}

fn worker(rx: mpsc::Receiver<Message>, events: mpsc::Sender<Message>) {
    let tts_dir = match engine::tts_root() {
        Some(d) => d,
        None => {
            eprintln!("[suflyor-tts] APPDATA not set — no voice dir");
            return;
        }
    };
    let voices = engine::scan_voices(&tts_dir);
    eprintln!(
        "[suflyor-tts] {} voice(s): {:?}",
        voices.len(),
        voices.iter().map(|v| v.id.as_str()).collect::<Vec<_>>()
    );

    let mut current_voice = engine::pick_voice_id(&voices, "");
    let mut engine_opt: Option<NeuralEngine> = None;
    let mut rate = 0i32;
    let mut playback_speed = PlaybackSpeed::default();
    let sid = 0;
    let mut next_id = 1_u64;
    let mut current: Option<(u64, Playback)> = None;
    let mut pending: VecDeque<String> = VecDeque::new();
    // Latency diagnostics: time from a SPEAK to its first audio chunk.
    let mut speak_t0: Option<std::time::Instant> = None;
    let mut announced = true;

    // Tell the parent we're alive.
    let mut out = std::io::stdout();
    let _ = writeln!(out, "READY");
    let _ = out.flush();

    loop {
        let message = if pending.is_empty() {
            match rx.recv() {
                Ok(c) => Some(c),
                Err(_) => break,
            }
        } else {
            match rx.try_recv() {
                Ok(c) => Some(c),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        };

        match message {
            Some(Message::Command(Cmd::Speak(text))) => {
                speak_t0 = Some(std::time::Instant::now());
                announced = false;
                if let Some((id, pb)) = current.take() {
                    pb.stop();
                    emit_playback_event(&mut out, "DONE", id);
                }
                pending.clear();
                let id = next_id;
                next_id = next_id.wrapping_add(1).max(1);
                emit_playback_event(&mut out, "STARTED", id);
                if engine_opt.is_none() {
                    if let Some(id) = current_voice.clone() {
                        match engine::load_voice(&tts_dir, &id) {
                            Ok(e) => {
                                eprintln!("[suflyor-tts] loaded '{id}' (sr={})", e.sample_rate());
                                engine_opt = Some(e);
                            }
                            Err(err) => eprintln!("[suflyor-tts] load '{id}' failed: {err:#}"),
                        }
                    }
                }
                if let Some(e) = &engine_opt {
                    let chunks = engine::text::chunk_text(&text);
                    if !chunks.is_empty() {
                        let notify = events.clone();
                        match Playback::start(
                            e.sample_rate(),
                            Some(Box::new(move || {
                                let _ = notify.send(Message::PlaybackDone(id));
                            })),
                        ) {
                            Ok(pb) => {
                                pb.set_speed(playback_speed.factor());
                                current = Some((id, pb));
                                pending = VecDeque::from(chunks);
                            }
                            Err(err) => {
                                eprintln!("[suflyor-tts] playback start failed: {err:#}");
                                emit_playback_failure(&mut out, id, "playback");
                            }
                        }
                    } else {
                        emit_playback_failure(&mut out, id, "empty");
                    }
                } else {
                    emit_playback_failure(&mut out, id, "voice");
                }
            }
            Some(Message::Command(Cmd::Pause)) => {
                if let Some((_, pb)) = &current {
                    pb.pause();
                }
            }
            Some(Message::Command(Cmd::Resume)) => {
                if let Some((_, pb)) = &current {
                    pb.resume();
                }
            }
            Some(Message::Command(Cmd::Seek(seconds))) => {
                if let Some((_, pb)) = &current {
                    pb.seek_seconds(seconds);
                }
            }
            Some(Message::Command(Cmd::SetPlaybackSpeed(percent))) => {
                let speed = playback_speed.set(percent);
                if let Some((_, pb)) = &current {
                    pb.set_speed(speed);
                }
            }
            Some(Message::Command(Cmd::Stop)) => {
                pending.clear();
                if let Some((id, pb)) = current.take() {
                    pb.stop();
                    emit_playback_event(&mut out, "DONE", id);
                }
            }
            Some(Message::Command(Cmd::SetRate(r))) => {
                rate = r.clamp(-10, 10);
            }
            Some(Message::Command(Cmd::SetVoice(id))) => {
                if let Some((active, pb)) = current.take() {
                    pb.stop();
                    emit_playback_event(&mut out, "DONE", active);
                }
                pending.clear();
                match engine::load_voice(&tts_dir, &id) {
                    Ok(e) => {
                        eprintln!("[suflyor-tts] switched to '{id}'");
                        engine_opt = Some(e);
                        current_voice = Some(id);
                    }
                    Err(err) => eprintln!("[suflyor-tts] switch '{id}' failed: {err:#}"),
                }
            }
            Some(Message::PlaybackDone(id)) => {
                if let Some(pb) = take_finished_playback(&mut current, id) {
                    pb.stop();
                    emit_playback_event(&mut out, "DONE", id);
                }
            }
            Some(Message::Shutdown) => break,
            None => match (&engine_opt, &current) {
                (Some(e), Some((_, pb))) => {
                    if let Some(chunk) = pending.pop_front() {
                        let speed = engine::rate_to_speed(rate);
                        match e.synth(&chunk, speed, sid) {
                            Ok(mut samples) => {
                                // Inter-chunk gap: chunks are concatenated with no
                                // silence between them, so the last word of one and
                                // the first of the next run together (the tester's
                                // "слова слепаются" / unnatural pauses). Append a
                                // short silence — but NOT after the final chunk.
                                if !pending.is_empty() {
                                    let gap = (e.sample_rate() as usize) * 15 / 100; // 150 ms
                                    samples.resize(samples.len() + gap, 0.0_f32);
                                }
                                pb.feed(samples);
                                if !announced {
                                    if let Some(t) = speak_t0 {
                                        eprintln!(
                                            "[suflyor-tts] first audio +{}ms",
                                            t.elapsed().as_millis()
                                        );
                                    }
                                    announced = true;
                                }
                            }
                            Err(err) => eprintln!("[suflyor-tts] synth failed: {err:#}"),
                        }
                        if pending.is_empty() {
                            pb.end_of_stream();
                        }
                    }
                }
                _ => pending.clear(),
            },
        }
    }

    // stdin closed (the app exited / was closed): STOP immediately so speech
    // does not keep playing after the app is gone (the tester hit read-aloud
    // continuing after closing the app).
    if let Some((_, pb)) = current.take() {
        pb.stop();
    }
}

fn emit_playback_event(out: &mut impl Write, kind: &str, id: u64) {
    let _ = writeln!(out, "{kind} id={id}");
    let _ = out.flush();
}

fn emit_playback_failure(out: &mut impl Write, id: u64, reason: &str) {
    let _ = writeln!(out, "FAILED id={id} reason={reason}");
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parses_bounded_seek_and_playback_speed() {
        assert!(matches!(parse_cmd("SEEK -10"), Some(Cmd::Seek(-10))));
        assert!(matches!(parse_cmd("SEEK 15"), Some(Cmd::Seek(15))));
        assert!(matches!(
            parse_cmd("SPEED 150"),
            Some(Cmd::SetPlaybackSpeed(150))
        ));
        assert!(parse_cmd("SEEK 31").is_none());
        assert!(parse_cmd("SPEED 301").is_none());
    }

    #[test]
    fn playback_speed_is_remembered_for_the_next_player() {
        let mut speed = PlaybackSpeed::default();
        assert_eq!(speed.factor(), 1.0);
        assert_eq!(speed.set(150), 1.5);
        assert_eq!(speed.factor(), 1.5);
    }

    #[test]
    fn playback_done_is_consumed_once_and_stale_ids_are_ignored() {
        let mut current = Some((7, "player"));
        assert_eq!(take_finished_playback(&mut current, 6), None);
        assert_eq!(take_finished_playback(&mut current, 7), Some("player"));
        assert_eq!(take_finished_playback(&mut current, 7), None);
    }
}
