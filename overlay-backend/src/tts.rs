//! Text-to-speech (read-aloud) — CLIENT for the `suflyor-tts` sidecar process.
//!
//! The neural engine (sherpa-onnx) cannot share a process with our `ort`/GigaAM
//! STT runtime — two statically-linked onnxruntimes collide and crash natively
//! on the second model load. So synthesis + playback live in a separate
//! `suflyor-tts.exe`; this module is a thin client that scans the installed
//! voices (for the Settings chooser) and forwards commands to the sidecar over
//! its stdin (SPEAK/PAUSE/RESUME/STOP/VOICE/RATE).
//!
//! The public API (`init`/`speak`/`pause`/`resume`/`stop`/`set_rate`/`set_voice`/
//! `voices`/`is_available`/[`VoiceInfo`]) is unchanged, so the tile 🔊/⏯ wiring
//! and the Settings panel don't care that the engine moved out of process.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child as Proc, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine as _;

/// Estimated unix-ms when the current read-aloud finishes playing (incl. a tail
/// cooldown). While `now < this`, [`is_speaking`] is true and the STT pipeline
/// suppresses transcription of BOTH the system loopback (the TTS heard back) AND
/// the microphone (the speakers' acoustic echo) — otherwise the read-aloud is
/// transcribed and either shown on the bar or answered by the AI (the tester's
/// "суфлёр слышит читалку и отвечает" loop + read-aloud text leaking onto the bar).
static SPEAKING_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

/// Current read rate (−10..+10), so the speaking-duration estimate scales with
/// speed (a slower rate = longer audio = longer suppression window).
static SPEAK_RATE: AtomicI32 = AtomicI32::new(0);

/// True while a read-aloud is (estimated to be) playing through the speakers.
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
    let until = (crate::journal::now_unix_ms() as u64).saturating_add((secs * 1000.0) as u64);
    SPEAKING_UNTIL_MS.store(until, Ordering::Release);
}

fn clear_speaking() {
    SPEAKING_UNTIL_MS.store(0, Ordering::Release);
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
            let collapsed = line.split_whitespace().collect::<Vec<_>>().join(" ");
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

/// The sidecar process + its stdin, plus the config to re-apply on respawn.
struct Sidecar {
    exe: PathBuf,
    voice: String,
    rate: i32,
    proc: Option<Proc>,
    stdin: Option<ChildStdin>,
}

impl Sidecar {
    /// Ensure a live child exists; (re)spawn if missing/dead and re-apply the
    /// selected voice + rate. Lazy: the first command spawns it, so an idle app
    /// that never reads anything aloud never starts the process.
    fn ensure(&mut self) {
        let alive = self
            .proc
            .as_mut()
            .map(|p| matches!(p.try_wait(), Ok(None)))
            .unwrap_or(false);
        if alive {
            return;
        }
        self.proc = None;
        self.stdin = None;
        if !self.exe.is_file() {
            log::warn!("tts: sidecar exe not found at {:?}", self.exe);
            return;
        }
        match spawn_sidecar(&self.exe) {
            Ok(mut child) => {
                self.stdin = child.stdin.take();
                self.proc = Some(child);
                if !self.voice.is_empty() {
                    let v = self.voice.clone();
                    self.write_raw(&format!("VOICE {v}"));
                }
                let r = self.rate;
                self.write_raw(&format!("RATE {r}"));
            }
            Err(e) => log::warn!("tts: failed to spawn sidecar: {e}"),
        }
    }

    /// Write a line without (re)spawning — used internally right after spawn.
    fn write_raw(&mut self, line: &str) {
        if let Some(si) = self.stdin.as_mut() {
            if writeln!(si, "{line}").and_then(|_| si.flush()).is_err() {
                self.stdin = None;
                self.proc = None;
            }
        }
    }

    /// Ensure the child is up, then send `line`.
    fn send(&mut self, line: &str) {
        self.ensure();
        self.write_raw(line);
    }
}

/// Handle to the TTS sidecar client. Cheap to clone.
#[derive(Clone)]
pub struct Tts {
    sidecar: Arc<Mutex<Sidecar>>,
    voices: Arc<Vec<VoiceInfo>>,
}

impl Tts {
    /// Build the client: scan installed voices and prepare (but don't yet spawn)
    /// the sidecar. `voice_id` empty/unknown → auto-pick a Russian voice.
    #[must_use]
    pub fn spawn(voice_id: Option<String>, rate: i32) -> Self {
        let voices = scan_installed_voices();
        let voice = pick_voice_id(&voices, &voice_id.unwrap_or_default()).unwrap_or_default();
        SPEAK_RATE.store(rate.clamp(-10, 10), Ordering::Release);
        let sidecar = Sidecar {
            exe: sidecar_exe_path(),
            voice,
            rate: rate.clamp(-10, 10),
            proc: None,
            stdin: None,
        };
        Self {
            sidecar: Arc::new(Mutex::new(sidecar)),
            voices: Arc::new(voices),
        }
    }

    /// True when at least one voice model is installed AND the sidecar exe is
    /// present (TTS usable).
    #[must_use]
    pub fn is_available(&self) -> bool {
        if self.voices.is_empty() {
            return false;
        }
        self.sidecar
            .lock()
            .map(|s| s.exe.is_file())
            .unwrap_or(false)
    }

    /// The installed voices, for the Settings chooser.
    #[must_use]
    pub fn voices(&self) -> &[VoiceInfo] {
        &self.voices
    }

    fn send(&self, line: String) {
        if let Ok(mut s) = self.sidecar.lock() {
            s.send(&line);
        }
    }

    /// Speak `text` now, interrupting any current speech. `text` may be markdown
    /// (a tile answer) — it is cleaned to spoken text first so the synthesizer
    /// voices words, not `**` / backticks / `#`.
    pub fn speak(&self, text: &str) {
        let spoken = crate::tts_normalize::normalize_for_speech(&speech_text::to_speech(text));
        if spoken.trim().is_empty() {
            return;
        }
        mark_speaking_for(spoken.chars().count());
        let b64 = base64::engine::general_purpose::STANDARD.encode(&spoken);
        self.send(format!("SPEAK {b64}"));
    }
    pub fn pause(&self) {
        self.send("PAUSE".to_string());
    }
    pub fn resume(&self) {
        self.send("RESUME".to_string());
    }
    pub fn stop(&self) {
        clear_speaking();
        self.send("STOP".to_string());
    }
    /// Set the read rate (−10…+10, 0 = normal). Applies to the next utterance.
    pub fn set_rate(&self, rate: i32) {
        let r = rate.clamp(-10, 10);
        SPEAK_RATE.store(r, Ordering::Release);
        if let Ok(mut s) = self.sidecar.lock() {
            s.rate = r;
            s.send(&format!("RATE {r}"));
        }
    }
    /// Switch the active voice by its [`VoiceInfo::id`] (the model dir name).
    pub fn set_voice(&self, id: &str) {
        if let Ok(mut s) = self.sidecar.lock() {
            s.voice = id.to_string();
            s.send(&format!("VOICE {id}"));
        }
    }

    /// Spawn the sidecar and preload the selected voice in the background, so the
    /// first `speak` doesn't pay the model-load latency. No-op if no voice is
    /// installed. The actual load happens inside the sidecar's own thread, so
    /// this returns immediately.
    pub fn warm(&self) {
        if self.voices.is_empty() {
            return;
        }
        if let Ok(mut s) = self.sidecar.lock() {
            s.ensure();
        }
    }
}

// ===== Process-global handle =====

static GLOBAL: std::sync::OnceLock<std::sync::Mutex<Tts>> = std::sync::OnceLock::new();

/// Initialize the global TTS client ONCE at startup (idempotent). `voice_id` /
/// `rate` come from config. Warms the sidecar (spawns it + preloads the voice in
/// the background) so the first 🔊 is prompt rather than paying a cold model load.
/// Safe to do eagerly: the sidecar has no `ort`, so there's no STT conflict.
pub fn init(voice_id: Option<String>, rate: i32) {
    let tts = Tts::spawn(voice_id, rate);
    tts.warm();
    let _ = GLOBAL.set(std::sync::Mutex::new(tts));
}

fn with<R>(f: impl FnOnce(&Tts) -> R) -> Option<R> {
    GLOBAL.get().and_then(|m| m.lock().ok()).map(|t| f(&t))
}

/// Speak `text` now (interrupts current speech). No-op if not initialized.
pub fn speak(text: &str) {
    with(|t| t.speak(text));
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
/// Preload the sidecar + voice in the background (called at startup by `init`).
pub fn warm() {
    with(|t| t.warm());
}
/// The installed voices (empty if TTS is unavailable / not yet initialized).
#[must_use]
pub fn voices() -> Vec<VoiceInfo> {
    // Re-scan the filesystem (not the init-time cache) so a voice installed
    // mid-session via the «Озвучка» install button appears in the chooser
    // without restarting the app.
    scan_installed_voices()
}
/// Whether at least one voice is installed and the sidecar is present. Re-scans
/// so it flips to true right after the install button finishes.
#[must_use]
pub fn is_available() -> bool {
    !scan_installed_voices().is_empty() && sidecar_exe_path().is_file()
}

// ===== Helpers (filesystem only — no sherpa/onnxruntime here) =====

/// Resolve `suflyor-tts.exe` next to the running executable. `pub(crate)` so the
/// diarization client (`crate::diarize`) spawns the SAME sidecar exe.
pub(crate) fn sidecar_exe_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("suflyor-tts.exe")))
        .unwrap_or_else(|| PathBuf::from("suflyor-tts.exe"))
}

/// Capture the sidecar's stderr (voice-load + synth + first-audio-latency
/// diagnostics) to `%APPDATA%\suflyor\suflyor-tts.log`, falling back to null.
fn sidecar_stderr() -> Stdio {
    if let Some(p) = crate::paths::data_root().map(|d| d.join("suflyor-tts.log")) {
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

#[cfg(windows)]
fn spawn_sidecar(exe: &Path) -> std::io::Result<Proc> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(sidecar_stderr())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
}

#[cfg(not(windows))]
fn spawn_sidecar(exe: &Path) -> std::io::Result<Proc> {
    Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(sidecar_stderr())
        .spawn()
}

/// `%APPDATA%\suflyor\tts`.
fn tts_root() -> Option<PathBuf> {
    crate::paths::data_root().map(|d| d.join("tts"))
}

/// Scan the installed voices for the chooser (a subdir with `*.onnx` +
/// `tokens.txt`). Pure filesystem — does not touch the engine.
fn scan_installed_voices() -> Vec<VoiceInfo> {
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
                name: friendly_name(name),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_is_clamped() {
        assert_eq!(20_i32.clamp(-10, 10), 10);
        assert_eq!((-20_i32).clamp(-10, 10), -10);
    }

    #[test]
    fn friendly_name_maps_known_voices() {
        assert_eq!(friendly_name("vits-piper-ru_RU-irina-medium"), "Ирина (ж)");
        assert_eq!(friendly_name("vits-piper-ru_RU-denis-medium"), "Денис (м)");
        assert_eq!(friendly_name("vits-mms-rus"), "MMS (рус)");
        assert_eq!(friendly_name("custom"), "custom");
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
}
