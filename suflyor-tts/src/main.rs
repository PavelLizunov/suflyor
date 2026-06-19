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
//!   PAUSE / RESUME / STOP
//! EOF on stdin (parent exits) → this process exits.

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
    SetRate(i32),
    SetVoice(String),
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
    // stdin → Cmd channel. Dropping `tx` on EOF makes the worker's recv() return
    // Err, which exits the process.
    let (tx, rx) = mpsc::channel::<Cmd>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if let Some(cmd) = parse_cmd(&line) {
                if tx.send(cmd).is_err() {
                    break;
                }
            }
        }
    });
    worker(rx);
}

fn worker(rx: mpsc::Receiver<Cmd>) {
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
    let sid = 0;
    let mut current: Option<Playback> = None;
    let mut pending: VecDeque<String> = VecDeque::new();
    // Latency diagnostics: time from a SPEAK to its first audio chunk.
    let mut speak_t0: Option<std::time::Instant> = None;
    let mut announced = true;

    // Tell the parent we're alive.
    let mut out = std::io::stdout();
    let _ = writeln!(out, "READY");
    let _ = out.flush();

    loop {
        let cmd = if pending.is_empty() {
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

        match cmd {
            Some(Cmd::Speak(text)) => {
                speak_t0 = Some(std::time::Instant::now());
                announced = false;
                if let Some(pb) = current.take() {
                    pb.stop();
                }
                pending.clear();
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
                        match Playback::start(e.sample_rate()) {
                            Ok(pb) => {
                                current = Some(pb);
                                pending = VecDeque::from(chunks);
                            }
                            Err(err) => eprintln!("[suflyor-tts] playback start failed: {err:#}"),
                        }
                    }
                }
            }
            Some(Cmd::Pause) => {
                if let Some(pb) = &current {
                    pb.pause();
                }
            }
            Some(Cmd::Resume) => {
                if let Some(pb) = &current {
                    pb.resume();
                }
            }
            Some(Cmd::Stop) => {
                pending.clear();
                if let Some(pb) = current.take() {
                    pb.stop();
                }
            }
            Some(Cmd::SetRate(r)) => {
                rate = r.clamp(-10, 10);
            }
            Some(Cmd::SetVoice(id)) => {
                if let Some(pb) = current.take() {
                    pb.stop();
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
            None => match (&engine_opt, &current) {
                (Some(e), Some(pb)) => {
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
    if let Some(pb) = current.take() {
        pb.stop();
    }
}
