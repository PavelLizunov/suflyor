//! Text-to-speech (read-aloud) — CLIENT for the `suflyor-tts` sidecar process.
//!
//! The neural engine (sherpa-onnx) cannot share a process with our `ort`/GigaAM
//! STT runtime — two statically-linked onnxruntimes collide and crash natively
//! on the second model load. So synthesis + playback live in a separate
//! `suflyor-tts.exe`; this module is a thin client that scans the installed
//! voices (for the Settings chooser) and forwards commands to the sidecar over
//! its stdin (SPEAK/PAUSE/RESUME/STOP/VOICE/RATE).
//!
//! The tile controls and Settings panel use this client API, so they don't care
//! that the engine moved out of process.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child as Proc, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicI32, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine as _;

// ===== Engine selection (RC17) =====

/// Read-aloud engine selection (config `tts_engine`). Diarization is NOT
/// affected — it always runs in the Piper sidecar (`suflyor-tts.exe`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// sherpa-onnx Piper voices in `suflyor-tts.exe` (the default).
    Piper,
    /// Experimental TeraTTSv2 ONNX graphs in `suflyor-teratts.exe`.
    Tera,
}

/// Parse config `tts_engine`: only the exact (case-insensitive) "tera"
/// selects the experimental engine; anything else stays on Piper.
#[must_use]
pub fn parse_engine(raw: &str) -> EngineKind {
    if raw.trim().eq_ignore_ascii_case("tera") {
        EngineKind::Tera
    } else {
        EngineKind::Piper
    }
}

/// Namespaced voice reference stored in config `tts_voice`: `piper:<dir>` or
/// `tera:<style>`. Legacy bare ids (pre-RC17 configs) resolve to Piper, so an
/// existing install keeps speaking with its saved voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceRef {
    pub engine: EngineKind,
    pub id: String,
}

#[must_use]
pub fn parse_voice_ref(raw: &str) -> VoiceRef {
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix("tera:") {
        VoiceRef {
            engine: EngineKind::Tera,
            id: rest.trim().to_string(),
        }
    } else if let Some(rest) = raw.strip_prefix("piper:") {
        VoiceRef {
            engine: EngineKind::Piper,
            id: rest.trim().to_string(),
        }
    } else {
        VoiceRef {
            engine: EngineKind::Piper,
            id: raw.to_string(),
        }
    }
}

#[must_use]
pub fn format_voice_ref(voice: &VoiceRef) -> String {
    let prefix = match voice.engine {
        EngineKind::Piper => "piper:",
        EngineKind::Tera => "tera:",
    };
    format!("{prefix}{}", voice.id)
}

/// Parsed READY handshake of the Tera sidecar stdout:
/// `READY engine=tera revision=<hex> voices=<a,b> sample_rate=44100 state=<s>`.
/// Old-style parsers that only prefix-match `READY` keep working.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeraReady {
    pub revision: String,
    pub voices: Vec<String>,
    pub sample_rate: u32,
    /// `ready` | `not-installed` | `error`.
    pub state: String,
}

/// Parse one stdout line of the Tera sidecar; None for anything that is not a
/// well-formed READY handshake.
#[must_use]
pub fn parse_ready_line(line: &str) -> Option<TeraReady> {
    let line = line.trim_end_matches(['\r', '\n']);
    if !line.starts_with("READY ") {
        return None;
    }
    let mut engine_ok = false;
    let mut revision = String::new();
    let mut voices = Vec::new();
    let mut sample_rate = 0u32;
    let mut state = String::new();
    for field in line.split_whitespace().skip(1) {
        let (key, value) = field.split_once('=')?;
        match key {
            "engine" => {
                if value != "tera" {
                    return None;
                }
                engine_ok = true;
            }
            "revision" => revision = value.to_string(),
            "voices" => {
                voices = value
                    .split(',')
                    .filter(|v| !v.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            "sample_rate" => sample_rate = value.parse().ok()?,
            "state" => state = value.to_string(),
            _ => {}
        }
    }
    engine_ok.then_some(TeraReady {
        revision,
        voices,
        sample_rate,
        state,
    })
}

/// Absolute unix-ms deadline for suppressing STT while read-aloud plays.
/// `u64::MAX` means playback is paused and may resume later.
static SPEAKING_UNTIL_MS: AtomicU64 = AtomicU64::new(0);
static PAUSED_REMAINING_MS: AtomicU64 = AtomicU64::new(0);

/// Current read rate (−10..+10), so the speaking-duration estimate scales with
/// speed (a slower rate = longer audio = longer suppression window).
static SPEAK_RATE: AtomicI32 = AtomicI32::new(0);

/// True while a read-aloud is (estimated to be) playing through the speakers,
/// OR is paused mid-utterance (playback will resume — keep suppressing STT).
#[must_use]
pub fn is_speaking() -> bool {
    (crate::journal::now_unix_ms() as u64) < SPEAKING_UNTIL_MS.load(Ordering::Acquire)
}

/// Map a read rate (−10..+10) to a speed multiplier (−10→0.5×, 0→1×, +10→2×) —
/// mirrors the sidecar's `engine::rate_to_speed` so the estimate tracks the
/// actual playback length.
fn rate_to_speed(rate: i32) -> f64 {
    2.0_f64.powf(f64::from(rate.clamp(-10, 10)) / 10.0)
}

/// Mark the read-aloud as playing for an estimated duration. The estimate is
/// deliberately GENEROUS — it errs toward suppressing STT slightly too long
/// rather than letting the TAIL of the speech leak into the transcript (the
/// tester saw read-aloud text appear on the bar AFTER playback):
/// `synth_latency + chars / (base_cps × speed) + tail_cooldown`. `base_cps`
/// (chars/sec at 1×) is intentionally low so the window over- rather than
/// under-shoots; the tail cooldown covers the loopback/mic buffer still in
/// flight after the audio actually stops. A new speak re-extends it; stop clears.
fn mark_speaking_for(chars: usize) {
    const BASE_CPS: f64 = 12.0;
    const SYNTH_LATENCY_S: f64 = 1.5;
    const TAIL_COOLDOWN_S: f64 = 2.0;
    let speed = rate_to_speed(SPEAK_RATE.load(Ordering::Acquire));
    let play_s = (chars as f64) / (BASE_CPS * speed).max(1.0);
    let secs = SYNTH_LATENCY_S + play_s + TAIL_COOLDOWN_S;
    PAUSED_REMAINING_MS.store(0, Ordering::Release);
    let until = (crate::journal::now_unix_ms() as u64).saturating_add((secs * 1000.0) as u64);
    SPEAKING_UNTIL_MS.store(until, Ordering::Release);
}

fn clear_speaking() {
    PAUSED_REMAINING_MS.store(0, Ordering::Release);
    SPEAKING_UNTIL_MS.store(0, Ordering::Release);
}

fn paused_remaining(until_ms: u64, now_ms: u64) -> Option<u64> {
    (until_ms != u64::MAX && now_ms < until_ms).then(|| until_ms - now_ms)
}

fn resumed_until(remaining_ms: u64, now_ms: u64) -> Option<u64> {
    (remaining_ms > 0).then(|| now_ms.saturating_add(remaining_ms))
}

fn pause_speaking() {
    let now = crate::journal::now_unix_ms() as u64;
    let until = SPEAKING_UNTIL_MS.load(Ordering::Acquire);
    if let Some(remaining) = paused_remaining(until, now) {
        PAUSED_REMAINING_MS.store(remaining, Ordering::Release);
        SPEAKING_UNTIL_MS.store(u64::MAX, Ordering::Release);
    }
}

fn resume_speaking() {
    let remaining = PAUSED_REMAINING_MS.swap(0, Ordering::AcqRel);
    if let Some(until) = resumed_until(remaining, crate::journal::now_unix_ms() as u64) {
        SPEAKING_UNTIL_MS.store(until, Ordering::Release);
    }
}

/// Markdown → spoken-text cleanup. Read-aloud must voice the WORDS, not the
/// markup: without this the TTS literally said "звёздочка звёздочка" for `**`,
/// read backticks for code, "решётка" for `#`, etc. (tester report on a tile
/// from the mic/PTT path, whose answer is full markdown). Strips block + inline
/// markdown and normalizes whitespace; plain text (selected text / OCR) passes
/// through essentially unchanged.
mod speech_text {
    /// Convert a markdown string into clean text for text-to-speech.
    pub fn to_speech(md: &str) -> String {
        let mut lines: Vec<String> = Vec::new();
        let mut fence = false;
        for raw in md.lines() {
            let t = raw.trim();
            // Code fence ``` / ~~~ — drop the fence line; keep the code inside as
            // plain text (reading the code beats reading the fence markers).
            if t.starts_with("```") || t.starts_with("~~~") {
                fence = !fence;
                continue;
            }
            if fence {
                lines.push(strip_inline(raw));
                continue;
            }
            // Horizontal rule (---, ***, ___) or a table separator (|---|:--:|).
            if is_rule(t) || is_table_separator(t) {
                continue;
            }
            // Block prefixes: heading #, blockquote >, list bullet, table pipes.
            let line = raw.trim_start();
            let line = line.trim_start_matches('#').trim_start();
            let line = line.trim_start_matches('>').trim_start();
            let unbulleted = strip_bullet(line).replace('|', " ");
            lines.push(strip_inline(&unbulleted));
        }
        normalize_ws(&lines.join("\n"))
    }

    fn is_rule(t: &str) -> bool {
        let bare: String = t.chars().filter(|c| !c.is_whitespace()).collect();
        bare.chars().count() >= 3
            && (bare.chars().all(|c| c == '-')
                || bare.chars().all(|c| c == '*')
                || bare.chars().all(|c| c == '_'))
    }

    fn is_table_separator(t: &str) -> bool {
        t.contains('|') && t.contains('-') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
    }

    /// Drop a leading list marker: "- ", "* ", "+ ", or an ordered "12. " / "12) ".
    fn strip_bullet(line: &str) -> &str {
        let l = line.trim_start();
        if let Some(rest) = l
            .strip_prefix("- ")
            .or_else(|| l.strip_prefix("* "))
            .or_else(|| l.strip_prefix("+ "))
        {
            return rest;
        }
        let digits: String = l.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() {
            let after = &l[digits.len()..];
            if let Some(rest) = after
                .strip_prefix(". ")
                .or_else(|| after.strip_prefix(") "))
            {
                return rest;
            }
        }
        line
    }

    /// Strip inline markdown: links → their text, and drop `*` / `` ` `` / `~`
    /// emphasis & code markers. `_` is LEFT intact — snake_case in dev text is
    /// common and would otherwise lose its underscores.
    fn strip_inline(s: &str) -> String {
        strip_links(s)
            .chars()
            .filter(|&c| c != '*' && c != '`' && c != '~')
            .collect()
    }

    /// Replace `[text](url)` / `![alt](url)` with just the text/alt (a spoken URL
    /// is noise). Anything that doesn't parse as a link is left untouched.
    fn strip_links(s: &str) -> String {
        let b: Vec<char> = s.chars().collect();
        let mut out = String::with_capacity(s.len());
        let mut i = 0;
        while i < b.len() {
            // An image link starts with '!'; skip it and parse the '[' that follows.
            let open = if b[i] == '!' && i + 1 < b.len() && b[i + 1] == '[' {
                i + 1
            } else {
                i
            };
            if b[open] == '[' {
                if let Some((text, next)) = parse_link(&b, open) {
                    out.push_str(&text);
                    i = next;
                    continue;
                }
            }
            out.push(b[i]);
            i += 1;
        }
        out
    }

    /// Parse `[text](url)` starting at the `[`; return (text, index-after-`)`).
    fn parse_link(b: &[char], open: usize) -> Option<(String, usize)> {
        let close = (open + 1..b.len()).find(|&j| b[j] == ']')?;
        if close + 1 >= b.len() || b[close + 1] != '(' {
            return None;
        }
        let paren_close = (close + 2..b.len()).find(|&j| b[j] == ')')?;
        let text: String = b[open + 1..close].iter().collect();
        Some((text, paren_close + 1))
    }

    /// Collapse intra-line whitespace to single spaces and runs of blank lines to
    /// one, then trim — gives the synthesizer clean word spacing.
    fn normalize_ws(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut blank_run = 0;
        for line in s.lines() {
            let collapsed = crate::text::collapse_ws(line);
            if collapsed.is_empty() {
                blank_run += 1;
                if blank_run <= 1 {
                    out.push('\n');
                }
            } else {
                blank_run = 0;
                out.push_str(&collapsed);
                out.push('\n');
            }
        }
        out.trim().to_string()
    }

    #[cfg(test)]
    mod tests {
        use super::to_speech;

        #[test]
        fn strips_emphasis_and_code() {
            assert_eq!(
                to_speech("Это **жирный** и `код` текст."),
                "Это жирный и код текст."
            );
            assert_eq!(to_speech("совсем *курсив* тут"), "совсем курсив тут");
        }

        #[test]
        fn strips_headings_bullets_links() {
            assert_eq!(
                to_speech("# Заголовок\n- пункт один\n- пункт два"),
                "Заголовок\nпункт один\nпункт два"
            );
            assert_eq!(
                to_speech("См. [ссылку](http://x.com) тут"),
                "См. ссылку тут"
            );
            assert_eq!(to_speech("1. первый\n2. второй"), "первый\nвторой");
        }

        #[test]
        fn drops_table_separator_and_rule() {
            let out = to_speech("| A | B |\n|---|---|\n| 1 | 2 |\n\n---\nдальше");
            assert!(!out.contains('|'), "pipes removed: {out:?}");
            assert!(!out.contains("---"), "rule/separator removed: {out:?}");
            assert!(out.contains('A') && out.contains("дальше"));
        }

        #[test]
        fn keeps_plain_text_and_underscores() {
            assert_eq!(
                to_speech("Привет, мир. Раз два три."),
                "Привет, мир. Раз два три."
            );
            assert_eq!(to_speech("snake_case_name"), "snake_case_name");
        }
    }
}

/// A selectable voice for the Settings chooser. `id` is the on-disk model dir
/// name (stable); `name` is the friendly display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
}

/// A Tera sidecar that crashes this many times in one session is declared
/// unusable; read-aloud falls back to Piper until the app restarts.
const TERA_CRASH_LIMIT: u32 = 3;

/// One engine's sidecar process + its stdin, plus the config to re-apply on
/// respawn. Piper and Tera share the line protocol, so one struct drives both.
struct Sidecar {
    kind: EngineKind,
    exe: PathBuf,
    /// Bare voice id inside this engine's namespace (Piper model dir name or
    /// Tera style id) — re-applied after every respawn.
    voice: String,
    rate: i32,
    /// Language tag for the Tera sidecar (ignored by Piper).
    lang: String,
    proc: Option<Proc>,
    stdin: Option<ChildStdin>,
    /// Respawns after a detected crash. Past [`TERA_CRASH_LIMIT`] the engine
    /// is bypassed for the rest of the session (fallback stays usable).
    crashes: u32,
    /// Latest READY handshake (Tera only; Piper stdout stays null).
    ready: Arc<Mutex<Option<TeraReady>>>,
}

impl Sidecar {
    /// Ensure a live child exists; (re)spawn if missing/dead and re-apply
    /// LANG/VOICE/RATE. Lazy: the first command spawns it, so an idle app that
    /// never reads anything aloud never starts the process. A child found DEAD
    /// here crashed since the last spawn (the only other exit is app shutdown,
    /// which never re-enters `ensure`).
    fn ensure(&mut self) {
        let alive = self
            .proc
            .as_mut()
            .map(|p| matches!(p.try_wait(), Ok(None)))
            .unwrap_or(false);
        if alive {
            return;
        }
        if self.proc.is_some() {
            self.crashes += 1;
            log::warn!("tts: {:?} sidecar died (crash {})", self.kind, self.crashes);
        }
        self.proc = None;
        self.stdin = None;
        if !self.exe.is_file() {
            log::warn!(
                "tts: {:?} sidecar exe not found at {:?}",
                self.kind,
                self.exe
            );
            return;
        }
        match spawn_engine_sidecar(&self.exe, self.kind) {
            Ok(mut child) => {
                if self.kind == EngineKind::Tera {
                    if let Some(stdout) = child.stdout.take() {
                        let slot = self.ready.clone();
                        let _ = std::thread::Builder::new()
                            .name("teratts-handshake".into())
                            .spawn(move || {
                                use std::io::BufRead as _;
                                let mut lines = std::io::BufReader::new(stdout).lines();
                                if let Some(Ok(first)) = lines.next() {
                                    if let Some(parsed) = parse_ready_line(&first) {
                                        if let Ok(mut guard) = slot.lock() {
                                            *guard = Some(parsed);
                                        }
                                    }
                                }
                                // STARTED/DONE/FAILED/REJECTED lines are drained
                                // and dropped: the host keeps its own speaking
                                // estimate and the protocol never carries text.
                                for _ in lines.by_ref() {}
                            });
                    }
                }
                self.stdin = child.stdin.take();
                self.proc = Some(child);
                if self.kind == EngineKind::Tera && !self.lang.is_empty() {
                    let lang = self.lang.clone();
                    self.write_raw(&format!("LANG {lang}"));
                }
                if !self.voice.is_empty() {
                    let v = self.voice.clone();
                    self.write_raw(&format!("VOICE {v}"));
                }
                let r = self.rate;
                self.write_raw(&format!("RATE {r}"));
            }
            Err(e) => log::warn!("tts: failed to spawn {:?} sidecar: {e}", self.kind),
        }
    }

    /// Too many crashes — bypass this engine for the rest of the session.
    fn crashed_out(&self) -> bool {
        self.crashes >= TERA_CRASH_LIMIT
    }

    /// Write a line without (re)spawning — used internally right after spawn.
    /// Reports whether the line actually reached the child's stdin: a broken
    /// pipe (dead/crashed sidecar) drops the handles and reports false so the
    /// caller can fall back instead of believing playback started.
    fn write_raw(&mut self, line: &str) -> bool {
        if let Some(si) = self.stdin.as_mut() {
            if writeln!(si, "{line}").and_then(|_| si.flush()).is_ok() {
                return true;
            }
        }
        self.stdin = None;
        self.proc = None;
        false
    }

    /// Ensure the child is up, then send `line`. Reports delivery.
    fn send(&mut self, line: &str) -> bool {
        self.ensure();
        self.write_raw(line)
    }

    /// Deliver `line` only if the child is ALIVE — never respawns. Control
    /// commands (PAUSE/RESUME/STOP) use this: respawning a dead sidecar just
    /// to deliver a control line would boot the whole engine for nothing (a
    /// Tera cold load is hundreds of MB of graphs), and a dead sidecar is not
    /// playing anything anyway. The dead child stays in `proc` so the next
    /// `ensure` still counts the crash.
    fn send_if_alive(&mut self, line: &str) -> bool {
        let alive = self
            .proc
            .as_mut()
            .map(|p| matches!(p.try_wait(), Ok(None)))
            .unwrap_or(false);
        alive && self.write_raw(line)
    }
}

const TARGET_NONE: u8 = 0;
const TARGET_PIPER: u8 = 1;
const TARGET_TERA: u8 = 2;

fn engine_code(kind: EngineKind) -> u8 {
    match kind {
        EngineKind::Piper => 0,
        EngineKind::Tera => 1,
    }
}

fn engine_from_code(code: u8) -> EngineKind {
    if code == 1 {
        EngineKind::Tera
    } else {
        EngineKind::Piper
    }
}

/// Handle to the TTS sidecar client. Cheap to clone. Holds BOTH engine
/// sidecars: the selected one speaks, and Piper stays the automatic fallback
/// whenever Tera is not installed, not ready, or crashes.
#[derive(Clone)]
pub struct Tts {
    /// Selected engine (config `tts_engine`).
    engine: Arc<AtomicU8>,
    /// Which sidecar accepted the last SPEAK — pause/resume/stop route there.
    last_target: Arc<AtomicU8>,
    piper: Arc<Mutex<Sidecar>>,
    tera: Arc<Mutex<Sidecar>>,
    /// Installed Piper voices (the Settings chooser lists the active engine's
    /// voices — see `voices`/`tera_voices`).
    voices: Arc<Vec<VoiceInfo>>,
}

impl Tts {
    /// Build the client: scan installed Piper voices and prepare (but don't
    /// yet spawn) both sidecars. `voice_raw` is the namespaced config value
    /// (`piper:<dir>` / `tera:<style>`; empty/unknown → engine default).
    #[must_use]
    pub fn spawn(engine: EngineKind, voice_raw: Option<String>, rate: i32, lang: &str) -> Self {
        let voices = scan_installed_voices(true);
        let vref = parse_voice_ref(&voice_raw.unwrap_or_default());
        let piper_configured = if vref.engine == EngineKind::Piper {
            vref.id.as_str()
        } else {
            ""
        };
        let piper_voice = pick_voice_id(&voices, piper_configured).unwrap_or_default();
        let tera_voice = if vref.engine == EngineKind::Tera {
            vref.id.clone()
        } else {
            String::new()
        };
        let rate = rate.clamp(-10, 10);
        SPEAK_RATE.store(rate, Ordering::Release);
        let piper = Sidecar {
            kind: EngineKind::Piper,
            exe: sidecar_exe_path(),
            voice: piper_voice,
            rate,
            lang: String::new(),
            proc: None,
            stdin: None,
            crashes: 0,
            ready: Arc::new(Mutex::new(None)),
        };
        let tera = Sidecar {
            kind: EngineKind::Tera,
            exe: tera_sidecar_exe_path(),
            voice: tera_voice,
            rate,
            lang: lang.to_string(),
            proc: None,
            stdin: None,
            crashes: 0,
            ready: Arc::new(Mutex::new(None)),
        };
        Self {
            engine: Arc::new(AtomicU8::new(engine_code(engine))),
            last_target: Arc::new(AtomicU8::new(TARGET_NONE)),
            piper: Arc::new(Mutex::new(piper)),
            tera: Arc::new(Mutex::new(tera)),
            voices: Arc::new(voices),
        }
    }

    /// The selected engine.
    #[must_use]
    pub fn engine_kind(&self) -> EngineKind {
        engine_from_code(self.engine.load(Ordering::Acquire))
    }

    /// Switch the selected engine at runtime (Settings). Warms the new target
    /// if it is usable.
    pub fn set_engine(&self, kind: EngineKind) {
        self.engine.store(engine_code(kind), Ordering::Release);
        self.warm();
    }

    /// Tera can speak right now: sidecar exe present, model fully installed,
    /// and not crashed-out this session.
    #[must_use]
    pub fn tera_usable(&self) -> bool {
        let process_ok = self
            .tera
            .lock()
            .map(|s| s.exe.is_file() && !s.crashed_out())
            .unwrap_or(false);
        process_ok
            && crate::teratts_install::installed_state()
                == crate::teratts_install::TeraInstalled::Ready
    }

    /// Latest READY handshake of the Tera sidecar (None until it spawned and
    /// answered). For the Settings status line + host tests.
    #[must_use]
    pub fn tera_ready(&self) -> Option<TeraReady> {
        self.tera
            .lock()
            .ok()
            .and_then(|s| s.ready.lock().ok().and_then(|g| g.clone()))
    }

    /// True when read-aloud can speak: the selected engine is usable, or the
    /// Piper fallback is (Tera selected but broken still leaves Piper).
    #[must_use]
    pub fn is_available(&self) -> bool {
        if self.engine_kind() == EngineKind::Tera && self.tera_usable() {
            return true;
        }
        available_on_disk()
    }

    /// The installed Piper voices, for the Settings chooser.
    #[must_use]
    pub fn voices(&self) -> &[VoiceInfo] {
        &self.voices
    }

    /// The pinned Tera voice styles, for the Settings chooser (installed or
    /// not — the install button appears when the model is missing).
    #[must_use]
    pub fn tera_voices() -> Vec<String> {
        crate::teratts_install::manifest()
            .map(|m| {
                let mut voices: Vec<String> = m
                    .files
                    .iter()
                    .filter_map(|f| {
                        let path = f.path.strip_prefix("styles/")?;
                        Some(path.split('/').next()?.to_string())
                    })
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                voices.sort();
                voices
            })
            .unwrap_or_default()
    }

    /// Send `line` to the target sidecar (respawning it if needed — used for
    /// commands that START work). Reports whether the line was delivered.
    fn send_to(&self, target: u8, line: &str) -> bool {
        let sidecar = match target {
            TARGET_TERA => &self.tera,
            _ => &self.piper,
        };
        sidecar.lock().map(|mut s| s.send(line)).unwrap_or(false)
    }

    /// Deliver a CONTROL line (PAUSE/RESUME/STOP) without ever respawning a
    /// dead sidecar. Reports delivery.
    fn control_to(&self, target: u8, line: &str) -> bool {
        let sidecar = match target {
            TARGET_TERA => &self.tera,
            _ => &self.piper,
        };
        sidecar
            .lock()
            .map(|mut s| s.send_if_alive(line))
            .unwrap_or(false)
    }

    /// Speak `text` now, interrupting any current speech. `text` may be
    /// markdown (a tile answer) — it is cleaned to spoken text first so the
    /// synthesizer voices words, not `**` / backticks / `#`.
    ///
    /// Engine selection + fallback: with `tts_engine = "tera"` the Tera
    /// sidecar speaks when usable AND its stdin accepts the SPEAK line; a
    /// missing engine OR a write failure (sidecar died mid-utterance) falls
    /// back to Piper within the same call. Returns whether playback was
    /// ACCEPTED — the STT suppression window is marked ONLY after a
    /// successful write, so a dead engine neither plays nor falsely silences
    /// the mic. Callers gate their "this tile is speaking" state on this.
    pub fn speak(&self, text: &str) -> bool {
        let spoken = crate::tts_normalize::normalize_for_speech(&speech_text::to_speech(text));
        if spoken.trim().is_empty() {
            return false;
        }
        let b64 = base64::engine::general_purpose::STANDARD.encode(&spoken);
        if self.engine_kind() == EngineKind::Tera {
            if self.tera_usable() {
                if self.send_to(TARGET_TERA, &format!("SPEAK {b64}")) {
                    mark_speaking_for(spoken.chars().count());
                    self.last_target.store(TARGET_TERA, Ordering::Release);
                    return true;
                }
                log::warn!("tts: Tera sidecar did not accept SPEAK — falling back to Piper");
            } else {
                log::warn!("tts: Tera engine unavailable — falling back to Piper");
            }
        }
        if !available_on_disk() {
            return false;
        }
        if !self.send_to(TARGET_PIPER, &format!("SPEAK {b64}")) {
            return false;
        }
        mark_speaking_for(spoken.chars().count());
        self.last_target.store(TARGET_PIPER, Ordering::Release);
        true
    }
    pub fn pause(&self) {
        // Preserve the remaining suppression before telling the sidecar to
        // pause, so a long pause can't let the deadline expire.
        pause_speaking();
        let target = self.last_target.load(Ordering::Acquire);
        if target != TARGET_NONE {
            self.control_to(target, "PAUSE");
        }
    }
    pub fn resume(&self) {
        resume_speaking();
        let target = self.last_target.load(Ordering::Acquire);
        if target != TARGET_NONE {
            self.control_to(target, "RESUME");
        }
    }
    pub fn stop(&self) {
        clear_speaking();
        let target = self.last_target.load(Ordering::Acquire);
        if target != TARGET_NONE && !self.control_to(target, "STOP") {
            // The sidecar is already gone — playback died with it. Never
            // respawn an engine just to deliver STOP to nothing.
            log::debug!("tts: STOP not delivered — target sidecar not running");
        }
    }
    /// Set the read rate (−10…+10, 0 = normal). Applies to the next utterance.
    /// Stored on both sidecars (survives respawn/fallback); sent live to the
    /// one that last spoke.
    pub fn set_rate(&self, rate: i32) {
        let r = rate.clamp(-10, 10);
        SPEAK_RATE.store(r, Ordering::Release);
        for sidecar in [&self.piper, &self.tera] {
            if let Ok(mut s) = sidecar.lock() {
                s.rate = r;
            }
        }
        let target = self.last_target.load(Ordering::Acquire);
        if target != TARGET_NONE {
            self.send_to(target, &format!("RATE {r}"));
        }
    }
    /// Switch the active voice by its NAMESPACED id (`piper:<dir>` or
    /// `tera:<style>`; a bare legacy id targets Piper).
    pub fn set_voice(&self, id: &str) {
        let vref = parse_voice_ref(id);
        let sidecar = match vref.engine {
            EngineKind::Piper => &self.piper,
            EngineKind::Tera => &self.tera,
        };
        if let Ok(mut s) = sidecar.lock() {
            s.voice = vref.id.clone();
            s.send(&format!("VOICE {}", vref.id));
        }
    }

    /// Spawn the SELECTED engine's sidecar and preload its voice in the
    /// background, so the first `speak` doesn't pay the model-load latency.
    /// Tera only warms when actually usable (a not-installed model must not
    /// spawn a failing sidecar); Piper needs an installed voice.
    pub fn warm(&self) {
        match self.engine_kind() {
            EngineKind::Tera => {
                if self.tera_usable() {
                    if let Ok(mut s) = self.tera.lock() {
                        s.ensure();
                    }
                }
            }
            EngineKind::Piper => {
                if !self.voices.is_empty() {
                    if let Ok(mut s) = self.piper.lock() {
                        s.ensure();
                    }
                }
            }
        }
    }
}

// ===== Process-global handle =====

static GLOBAL: std::sync::OnceLock<std::sync::Mutex<Tts>> = std::sync::OnceLock::new();

/// Initialize the global TTS client ONCE at startup (idempotent). `engine` /
/// `voice_id` / `rate` come from config; `lang` ("ru"/"en") tags Tera speech.
/// Warms the selected sidecar (spawns it + preloads the voice in the
/// background) so the first speak is prompt rather than paying a cold model
/// load. Safe to do eagerly: the sidecars carry their own onnxruntimes, never
/// sharing the host's `ort`/GigaAM binary.
pub fn init(engine: Option<String>, voice_id: Option<String>, rate: i32, lang: &str) {
    let tts = Tts::spawn(
        parse_engine(&engine.unwrap_or_default()),
        voice_id,
        rate,
        lang,
    );
    tts.warm();
    let _ = GLOBAL.set(std::sync::Mutex::new(tts));
}

fn with<R>(f: impl FnOnce(&Tts) -> R) -> Option<R> {
    GLOBAL.get().and_then(|m| m.lock().ok()).map(|t| f(&t))
}

/// Speak `text` now (interrupts current speech). Returns whether playback was
/// ACCEPTED (TTS available + non-empty text) — the STT suppression window is
/// marked only then. False if TTS is uninitialized or unavailable. Callers gate
/// their visible "speaking" state on this so a missing engine isn't shown usable.
pub fn speak(text: &str) -> bool {
    with(|t| t.speak(text)).unwrap_or(false)
}
pub fn pause() {
    with(|t| t.pause());
}
pub fn resume() {
    with(|t| t.resume());
}
pub fn stop() {
    with(|t| t.stop());
}
pub fn set_rate(rate: i32) {
    with(|t| t.set_rate(rate));
}
pub fn set_voice(id: &str) {
    with(|t| t.set_voice(id));
}
/// Switch the selected read-aloud engine at runtime (Settings changes).
pub fn set_engine(engine_raw: &str) {
    let kind = parse_engine(engine_raw);
    with(|t| t.set_engine(kind));
}
/// The engine currently selected for read-aloud.
#[must_use]
pub fn active_engine() -> EngineKind {
    with(|t| t.engine_kind()).unwrap_or(EngineKind::Piper)
}
/// Latest READY handshake of the Tera sidecar (Settings status + tests).
#[must_use]
pub fn tera_ready() -> Option<TeraReady> {
    with(|t| t.tera_ready()).flatten()
}
/// True when the Tera engine can speak right now (exe + installed model +
/// not crashed-out).
#[must_use]
pub fn tera_usable() -> bool {
    with(|t| t.tera_usable()).unwrap_or(false)
}
/// The pinned Tera voice style ids for the Settings chooser.
#[must_use]
pub fn tera_voice_ids() -> Vec<String> {
    Tts::tera_voices()
}
/// Preload the sidecar + voice in the background (called at startup by `init`).
pub fn warm() {
    with(|t| t.warm());
}
/// The installed voices (empty if TTS is unavailable / not yet initialized).
/// `ru` picks the display language of the friendly labels; ids are stable.
#[must_use]
pub fn voices(ru: bool) -> Vec<VoiceInfo> {
    // Re-scan the filesystem (not the init-time cache) so a voice installed
    // mid-session via the «Озвучка» install button appears in the chooser
    // without restarting the app.
    scan_installed_voices(ru)
}
/// Whether at least one voice is installed and the sidecar is present. Re-scans
/// so it flips to true right after the install button finishes.
#[must_use]
pub fn is_available() -> bool {
    available_on_disk()
}

fn available_on_disk() -> bool {
    !scan_installed_voices(true).is_empty() && sidecar_exe_path().is_file()
}

// ===== Helpers (filesystem only — no sherpa/onnxruntime here) =====

/// Resolve `suflyor-tts.exe` next to the running executable — the PIPER
/// read-aloud path. The diarization client deliberately resolves the same exe
/// through its OWN helper (`crate::diarize::diarization_exe_path`) so the
/// read-aloud and diarization sidecar paths stay independent even when
/// read-aloud moves to the Tera sidecar.
pub(crate) fn sidecar_exe_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("suflyor-tts.exe")))
        .unwrap_or_else(|| PathBuf::from("suflyor-tts.exe"))
}

/// Resolve `suflyor-teratts.exe` (experimental TeraTTSv2 read-aloud sidecar).
/// Never used by diarization.
pub(crate) fn tera_sidecar_exe_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("suflyor-teratts.exe")))
        .unwrap_or_else(|| PathBuf::from("suflyor-teratts.exe"))
}

/// Capture a sidecar's stderr (voice-load + synth + first-audio-latency
/// diagnostics) to its own log under `%APPDATA%\suflyor\`, falling back to
/// null. Logs never contain spoken text — the sidecars only print counts/ids.
fn sidecar_stderr(kind: EngineKind) -> Stdio {
    let name = match kind {
        EngineKind::Piper => "suflyor-tts.log",
        EngineKind::Tera => "suflyor-teratts.log",
    };
    if let Some(p) = crate::paths::data_root().map(|d| d.join(name)) {
        if let Ok(f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
        {
            return Stdio::from(f);
        }
    }
    Stdio::null()
}

fn spawn_engine_sidecar(exe: &Path, kind: EngineKind) -> std::io::Result<Proc> {
    let mut cmd = Command::new(exe);
    // Tera's stdout carries the status handshake (READY/STARTED/DONE/FAILED);
    // Piper keeps the legacy null stdout.
    let stdout = if kind == EngineKind::Tera {
        Stdio::piped()
    } else {
        Stdio::null()
    };
    cmd.stdin(Stdio::piped())
        .stdout(stdout)
        .stderr(sidecar_stderr(kind));
    crate::download::no_window(&mut cmd).spawn()
}

/// `%APPDATA%\suflyor\tts`.
fn tts_root() -> Option<PathBuf> {
    crate::paths::data_root().map(|d| d.join("tts"))
}

/// Scan the installed voices for the chooser (a subdir with `*.onnx` +
/// `tokens.txt`). Pure filesystem — does not touch the engine.
fn scan_installed_voices(ru: bool) -> Vec<VoiceInfo> {
    let Some(tts_dir) = tts_root() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&tts_dir) else {
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
        if has_onnx(&path) && path.join("tokens.txt").is_file() {
            out.push(VoiceInfo {
                id: name.to_string(),
                name: friendly_name(name, ru),
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn has_onnx(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries
        .flatten()
        .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("onnx"))
}

/// Choose which voice id to load: configured if installed, else Irina → any
/// Piper → any Russian → first installed. Public so the Settings "Озвучка"
/// dropdown can show the SAME voice the engine actually resolves to when the
/// saved `tts_voice` is empty or points at an uninstalled voice (otherwise the
/// label would show `voices[0]`, which differs from the engine's preference).
#[must_use]
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

fn friendly_name(dir: &str, ru: bool) -> String {
    let d = dir.to_lowercase();
    if ru {
        if d.contains("irina") {
            return "Ирина (ж)".to_string();
        }
        if d.contains("ruslan") {
            return "Руслан (м)".to_string();
        }
        if d.contains("dmitri") {
            return "Дмитрий (м)".to_string();
        }
        if d.contains("denis") {
            return "Денис (м)".to_string();
        }
        if d.contains("mms") {
            return "MMS (рус)".to_string();
        }
    } else {
        if d.contains("irina") {
            return "Irina (F)".to_string();
        }
        if d.contains("ruslan") {
            return "Ruslan (M)".to_string();
        }
        if d.contains("dmitri") {
            return "Dmitri (M)".to_string();
        }
        if d.contains("denis") {
            return "Denis (M)".to_string();
        }
        if d.contains("mms") {
            return "MMS (RU)".to_string();
        }
    }
    dir.to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn pause_and_resume_preserve_a_finite_deadline() {
        assert_eq!(paused_remaining(11_000, 3_000), Some(8_000));
        assert_eq!(resumed_until(8_000, 50_000), Some(58_000));
        assert_eq!(paused_remaining(11_000, 11_000), None);
        assert_eq!(paused_remaining(u64::MAX, 3_000), None);
        assert_eq!(resumed_until(0, 50_000), None);
    }

    #[test]
    fn rate_is_clamped() {
        assert_eq!(20_i32.clamp(-10, 10), 10);
        assert_eq!((-20_i32).clamp(-10, 10), -10);
    }

    #[test]
    fn friendly_name_maps_known_voices() {
        assert_eq!(
            friendly_name("vits-piper-ru_RU-irina-medium", true),
            "Ирина (ж)"
        );
        assert_eq!(
            friendly_name("vits-piper-ru_RU-denis-medium", true),
            "Денис (м)"
        );
        assert_eq!(friendly_name("vits-mms-rus", true), "MMS (рус)");
        assert_eq!(friendly_name("custom", true), "custom");
        // English UI: same voices, Latin labels (they used to stay Cyrillic).
        assert_eq!(
            friendly_name("vits-piper-ru_RU-irina-medium", false),
            "Irina (F)"
        );
        assert_eq!(
            friendly_name("vits-piper-ru_RU-ruslan-medium", false),
            "Ruslan (M)"
        );
        assert_eq!(friendly_name("vits-mms-rus", false), "MMS (RU)");
        assert_eq!(friendly_name("custom", false), "custom");
    }

    #[test]
    fn pick_voice_prefers_irina_then_first() {
        let voices = vec![
            VoiceInfo {
                id: "vits-mms-rus".into(),
                name: "MMS (рус)".into(),
            },
            VoiceInfo {
                id: "vits-piper-ru_RU-irina-medium".into(),
                name: "Ирина (ж)".into(),
            },
        ];
        assert_eq!(
            pick_voice_id(&voices, "").as_deref(),
            Some("vits-piper-ru_RU-irina-medium")
        );
        assert_eq!(
            pick_voice_id(&voices, "vits-mms-rus").as_deref(),
            Some("vits-mms-rus")
        );
        assert!(pick_voice_id(&[], "").is_none());
    }

    #[test]
    fn speak_encodes_base64() {
        // The wire format must round-trip arbitrary text (incl. newlines).
        let text = "Привет!\nВторая строка.";
        let b64 = base64::engine::general_purpose::STANDARD.encode(text);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&b64)
            .ok()
            .and_then(|b| String::from_utf8(b).ok());
        assert_eq!(decoded.as_deref(), Some(text));
    }

    // ===== RC17: engine selection, namespaces, handshake, fallback =====

    #[test]
    fn engine_selection_defaults_to_piper() {
        assert_eq!(parse_engine("tera"), EngineKind::Tera);
        assert_eq!(parse_engine("TERA "), EngineKind::Tera);
        assert_eq!(parse_engine("piper"), EngineKind::Piper);
        assert_eq!(parse_engine(""), EngineKind::Piper);
        assert_eq!(parse_engine("garbage"), EngineKind::Piper);
    }

    #[test]
    fn voice_refs_namespace_and_legacy_compat() {
        assert_eq!(
            parse_voice_ref("tera:ru_f1"),
            VoiceRef {
                engine: EngineKind::Tera,
                id: "ru_f1".into()
            }
        );
        assert_eq!(
            parse_voice_ref("piper:vits-piper-ru_RU-irina-medium"),
            VoiceRef {
                engine: EngineKind::Piper,
                id: "vits-piper-ru_RU-irina-medium".into()
            }
        );
        // Legacy bare id (pre-RC17 config) resolves to Piper.
        assert_eq!(
            parse_voice_ref("vits-piper-ru_RU-irina-medium").engine,
            EngineKind::Piper
        );
        assert_eq!(
            format_voice_ref(&parse_voice_ref("tera:ru_f1")),
            "tera:ru_f1"
        );
        assert_eq!(
            format_voice_ref(&parse_voice_ref("piper:irina")),
            "piper:irina"
        );
    }

    #[test]
    fn ready_handshake_parses_capabilities() {
        let line = "READY engine=tera revision=f05ea799 voices=ru_f1,ru_m5 \
                    sample_rate=44100 state=ready";
        let ready = parse_ready_line(line).unwrap();
        assert_eq!(ready.revision, "f05ea799");
        assert_eq!(ready.voices, vec!["ru_f1".to_string(), "ru_m5".to_string()]);
        assert_eq!(ready.sample_rate, 44100);
        assert_eq!(ready.state, "ready");
        // Not-installed state with no voices still parses.
        let empty = parse_ready_line(
            "READY engine=tera revision=abc voices= sample_rate=44100 state=not-installed",
        )
        .unwrap();
        assert!(empty.voices.is_empty());
        assert_eq!(empty.state, "not-installed");
    }

    #[test]
    fn ready_handshake_rejects_foreign_lines() {
        // Legacy suflyor-tts handshake: still just "READY" — must NOT parse
        // as a Tera handshake.
        assert!(parse_ready_line("READY").is_none());
        assert!(parse_ready_line("READY engine=piper").is_none());
        assert!(parse_ready_line("STARTED id=1").is_none());
        assert!(parse_ready_line("").is_none());
        // Missing a required key=value field → malformed.
        assert!(
            parse_ready_line("READY engine=tera revision=x sample_rate=oops state=ready").is_none()
        );
    }

    #[test]
    fn tera_engine_falls_back_when_sidecar_missing() {
        // Test binaries have no suflyor-teratts.exe next to them, so Tera is
        // unusable and a Tera-selected client must NOT report itself usable —
        // the Piper fallback path decides availability.
        let _lock = SPEAKING_GLOBAL_LOCK.lock().unwrap();
        clear_speaking();
        let tts = Tts::spawn(EngineKind::Tera, Some("tera:ru_f1".into()), 0, "ru");
        assert_eq!(tts.engine_kind(), EngineKind::Tera);
        assert!(!tts.tera_usable());
        assert!(!tts.speak("Привет"));
        // A rejected speak must NOT mark the STT suppression window — errors
        // never falsely mark playback as active.
        assert!(!is_speaking());
        tts.set_engine(EngineKind::Piper);
        assert_eq!(tts.engine_kind(), EngineKind::Piper);
    }

    fn missing_exe_sidecar(kind: EngineKind) -> Sidecar {
        Sidecar {
            kind,
            exe: PathBuf::from("definitely-missing-sidecar.exe"),
            voice: String::new(),
            rate: 0,
            lang: "ru".into(),
            proc: None,
            stdin: None,
            crashes: 0,
            ready: Arc::new(Mutex::new(None)),
        }
    }

    /// The speaking-window statics are process-global; tests that touch them
    /// serialize here so parallel test threads cannot interleave.
    static SPEAKING_GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn send_reports_failure_when_the_sidecar_cannot_run() {
        // Write-failure P2: a SPEAK into an un-runnable sidecar must report
        // false (the caller falls back / skips the suppression window)
        // instead of silently pretending the line was delivered.
        let mut sidecar = missing_exe_sidecar(EngineKind::Tera);
        assert!(!sidecar.send("SPEAK aaa"));
        assert!(sidecar.proc.is_none());
        assert!(sidecar.stdin.is_none());
    }

    #[test]
    fn control_commands_never_respawn_a_dead_sidecar() {
        // STOP-dead-sidecar P2: PAUSE/RESUME/STOP deliver only to a LIVE
        // child — a dead/never-spawned sidecar gets no respawn (no pointless
        // engine boot), and the failure is reported so callers can log it.
        let mut sidecar = missing_exe_sidecar(EngineKind::Tera);
        assert!(!sidecar.send_if_alive("STOP"));
        assert!(!sidecar.send_if_alive("PAUSE"));
        assert!(!sidecar.send_if_alive("RESUME"));
        assert!(sidecar.proc.is_none(), "control lines must not spawn");
    }

    #[test]
    fn stop_clears_speaking_even_when_no_sidecar_runs() {
        // The suppression estimate is cleared regardless of delivery, so a
        // dead sidecar cannot leave the mic suppressed.
        let _lock = SPEAKING_GLOBAL_LOCK.lock().unwrap();
        mark_speaking_for(100);
        assert!(is_speaking());
        let tts = Tts::spawn(EngineKind::Piper, None, 0, "ru");
        tts.stop();
        assert!(!is_speaking());
    }

    #[test]
    fn crash_limit_bypasses_the_engine() {
        let mut sidecar = Sidecar {
            kind: EngineKind::Tera,
            exe: PathBuf::from("missing.exe"),
            voice: String::new(),
            rate: 0,
            lang: "ru".into(),
            proc: None,
            stdin: None,
            crashes: 0,
            ready: Arc::new(Mutex::new(None)),
        };
        assert!(!sidecar.crashed_out());
        sidecar.crashes = TERA_CRASH_LIMIT;
        assert!(sidecar.crashed_out());
    }
}
