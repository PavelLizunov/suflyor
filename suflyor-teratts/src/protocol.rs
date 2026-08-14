//! Stdin/stdout line protocol shared with the host.
//!
//! Stdin commands mirror suflyor-tts byte-for-byte (VOICE/RATE/SPEAK/PAUSE/
//! RESUME/STOP/SEEK/SPEED) so one host driver can talk to either sidecar. `LANG` is an
//! additive extension: old hosts never send it, and the default stays `ru`.
//! Unknown or malformed lines are never silently dropped — the sidecar answers
//! with a `REJECTED` status line so host tests can observe the refusal.
//!
//! Stdout is a status handshake, one ASCII line per event:
//!   READY engine=tera revision=<hex> voices=<a,b,c> sample_rate=44100
//!   STARTED id=<n>
//!   PLAYING id=<n>
//!   DONE id=<n>
//!   FAILED id=<n> reason=<token>
//!   REJECTED reason=<token>
//! Status lines never contain request text, voices' spoken content, or
//! credentials — ids, fixed tokens, and counts only.

/// Recognized stdin commands.
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    /// Synthesize + play base64-decoded UTF-8 text, interrupting current audio.
    Speak(String),
    Pause,
    Resume,
    Stop,
    /// Relative seek in seconds. The player clamps to retained/buffered PCM.
    Seek(i32),
    /// Pitch-preserving playback speed as an integer percent (50..=300).
    SetPlaybackSpeed(i32),
    /// Read rate −10..=10, mapped onto Tera `duration_scale` (higher rate →
    /// shorter durations).
    SetRate(i32),
    /// Select a Tera voice style by id (e.g. `ru_f1`).
    SetVoice(String),
    /// Language tag applied to untagged SPEAK text: `ru` or `en`.
    SetLang(String),
}

/// Why a stdin line was rejected. `UnknownVoice` also serves as the FAILED
/// reason token when a VOICE switch or SPEAK targets a voice that is not
/// installed in the pinned release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    InvalidCommand,
    InvalidBase64,
    InvalidUtf8,
    InvalidRate,
    InvalidSeek,
    InvalidSpeed,
    InvalidLang,
    EmptyVoice,
    UnknownVoice,
}

impl RejectReason {
    pub fn token(self) -> &'static str {
        match self {
            RejectReason::InvalidCommand => "invalid-command",
            RejectReason::InvalidBase64 => "invalid-base64",
            RejectReason::InvalidUtf8 => "invalid-utf8",
            RejectReason::InvalidRate => "invalid-rate",
            RejectReason::InvalidSeek => "invalid-seek",
            RejectReason::InvalidSpeed => "invalid-speed",
            RejectReason::InvalidLang => "invalid-lang",
            RejectReason::EmptyVoice => "empty-voice",
            RejectReason::UnknownVoice => "unknown-voice",
        }
    }
}

/// Parse one stdin line. `Ok(None)` means the line is blank and ignored;
/// `Err(reason)` must surface as a REJECTED status line.
pub fn parse_cmd(line: &str) -> Result<Option<Cmd>, RejectReason> {
    use base64::Engine as _;
    let line = line.trim_end_matches(['\r', '\n']);
    if line.trim().is_empty() {
        return Ok(None);
    }
    match line {
        "PAUSE" => return Ok(Some(Cmd::Pause)),
        "RESUME" => return Ok(Some(Cmd::Resume)),
        "STOP" => return Ok(Some(Cmd::Stop)),
        _ => {}
    }
    if let Some(rest) = line.strip_prefix("RATE ") {
        let rate = rest
            .trim()
            .parse::<i32>()
            .map_err(|_| RejectReason::InvalidRate)?;
        if !(-10..=10).contains(&rate) {
            return Err(RejectReason::InvalidRate);
        }
        return Ok(Some(Cmd::SetRate(rate)));
    }
    if let Some(rest) = line.strip_prefix("SEEK ") {
        let seconds = rest
            .trim()
            .parse::<i32>()
            .map_err(|_| RejectReason::InvalidSeek)?;
        if !(-30..=30).contains(&seconds) {
            return Err(RejectReason::InvalidSeek);
        }
        return Ok(Some(Cmd::Seek(seconds)));
    }
    if let Some(rest) = line.strip_prefix("SPEED ") {
        let percent = rest
            .trim()
            .parse::<i32>()
            .map_err(|_| RejectReason::InvalidSpeed)?;
        if !(50..=300).contains(&percent) {
            return Err(RejectReason::InvalidSpeed);
        }
        return Ok(Some(Cmd::SetPlaybackSpeed(percent)));
    }
    if let Some(rest) = line.strip_prefix("VOICE ") {
        let id = rest.trim();
        if id.is_empty() {
            return Err(RejectReason::EmptyVoice);
        }
        return Ok(Some(Cmd::SetVoice(id.to_string())));
    }
    if let Some(rest) = line.strip_prefix("LANG ") {
        let lang = rest.trim();
        if lang != "ru" && lang != "en" {
            return Err(RejectReason::InvalidLang);
        }
        return Ok(Some(Cmd::SetLang(lang.to_string())));
    }
    if let Some(rest) = line.strip_prefix("SPEAK ") {
        let payload = rest.trim();
        if payload.is_empty() {
            return Err(RejectReason::InvalidBase64);
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|_| RejectReason::InvalidBase64)?;
        let text = String::from_utf8(bytes).map_err(|_| RejectReason::InvalidUtf8)?;
        return Ok(Some(Cmd::Speak(text)));
    }
    Err(RejectReason::InvalidCommand)
}

/// Stdout status events. `id` is the utterance generation of a SPEAK.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Ready {
        revision: String,
        voices: Vec<String>,
        sample_rate: u32,
        /// `ready`, `not-installed`, or `error`. Hosts that only understand
        /// the legacy suflyor-tts handshake still match the `READY` prefix.
        state: String,
    },
    Started {
        id: u64,
    },
    Playing {
        id: u64,
    },
    Done {
        id: u64,
    },
    Failed {
        id: u64,
        reason: String,
    },
    Rejected {
        reason: RejectReason,
    },
}

impl Event {
    /// Render as one protocol line (no trailing newline).
    pub fn to_line(&self) -> String {
        match self {
            Event::Ready {
                revision,
                voices,
                sample_rate,
                state,
            } => format!(
                "READY engine=tera revision={revision} voices={} sample_rate={sample_rate} state={state}",
                voices.join(",")
            ),
            Event::Started { id } => format!("STARTED id={id}"),
            Event::Playing { id } => format!("PLAYING id={id}"),
            Event::Done { id } => format!("DONE id={id}"),
            Event::Failed { id, reason } => format!("FAILED id={id} reason={reason}"),
            Event::Rejected { reason } => format!("REJECTED reason={}", reason.token()),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn b64(text: &str) -> String {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.encode(text.as_bytes())
    }

    #[test]
    fn parses_speak_with_valid_base64() {
        let cmd = parse_cmd(&format!("SPEAK {}", b64("привет"))).unwrap();
        assert_eq!(cmd, Some(Cmd::Speak("привет".into())));
    }

    #[test]
    fn rejects_invalid_base64() {
        assert_eq!(
            parse_cmd("SPEAK !!!not-base64!!!").unwrap_err(),
            RejectReason::InvalidBase64
        );
        assert_eq!(
            parse_cmd("SPEAK ").unwrap_err(),
            RejectReason::InvalidBase64
        );
    }

    #[test]
    fn rejects_base64_that_is_not_utf8() {
        use base64::Engine as _;
        let raw = base64::engine::general_purpose::STANDARD.encode([0xff_u8, 0xfe, 0xfd]);
        assert_eq!(
            parse_cmd(&format!("SPEAK {raw}")).unwrap_err(),
            RejectReason::InvalidUtf8
        );
    }

    #[test]
    fn parses_control_commands() {
        assert_eq!(parse_cmd("PAUSE").unwrap(), Some(Cmd::Pause));
        assert_eq!(parse_cmd("RESUME").unwrap(), Some(Cmd::Resume));
        assert_eq!(parse_cmd("STOP").unwrap(), Some(Cmd::Stop));
        assert_eq!(parse_cmd("STOP\r\n").unwrap(), Some(Cmd::Stop));
    }

    #[test]
    fn parses_rate_and_rejects_out_of_range() {
        assert_eq!(parse_cmd("RATE -10").unwrap(), Some(Cmd::SetRate(-10)));
        assert_eq!(parse_cmd("RATE 10").unwrap(), Some(Cmd::SetRate(10)));
        assert_eq!(parse_cmd("RATE 11").unwrap_err(), RejectReason::InvalidRate);
        assert_eq!(
            parse_cmd("RATE abc").unwrap_err(),
            RejectReason::InvalidRate
        );
    }

    #[test]
    fn parses_seek_and_playback_speed_with_strict_bounds() {
        assert_eq!(parse_cmd("SEEK -10").unwrap(), Some(Cmd::Seek(-10)));
        assert_eq!(parse_cmd("SEEK 15").unwrap(), Some(Cmd::Seek(15)));
        assert_eq!(
            parse_cmd("SPEED 150").unwrap(),
            Some(Cmd::SetPlaybackSpeed(150))
        );
        assert_eq!(parse_cmd("SEEK 31").unwrap_err(), RejectReason::InvalidSeek);
        assert_eq!(
            parse_cmd("SPEED 301").unwrap_err(),
            RejectReason::InvalidSpeed
        );
    }

    #[test]
    fn parses_voice_and_lang() {
        assert_eq!(
            parse_cmd("VOICE ru_f1").unwrap(),
            Some(Cmd::SetVoice("ru_f1".into()))
        );
        assert_eq!(parse_cmd("VOICE ").unwrap_err(), RejectReason::EmptyVoice);
        assert_eq!(
            parse_cmd("LANG ru").unwrap(),
            Some(Cmd::SetLang("ru".into()))
        );
        assert_eq!(parse_cmd("LANG de").unwrap_err(), RejectReason::InvalidLang);
    }

    #[test]
    fn unknown_commands_are_rejected() {
        assert_eq!(
            parse_cmd("SHUTDOWN").unwrap_err(),
            RejectReason::InvalidCommand
        );
        assert_eq!(
            parse_cmd("SPEAKX abc").unwrap_err(),
            RejectReason::InvalidCommand
        );
    }

    #[test]
    fn blank_lines_are_ignored() {
        assert_eq!(parse_cmd("").unwrap(), None);
        assert_eq!(parse_cmd("  \n").unwrap(), None);
    }

    #[test]
    fn event_lines_are_ascii_and_stable() {
        let ready = Event::Ready {
            revision: "abc123".into(),
            voices: vec!["ru_f1".into(), "ru_m5".into()],
            sample_rate: 44100,
            state: "ready".into(),
        }
        .to_line();
        assert_eq!(
            ready,
            "READY engine=tera revision=abc123 voices=ru_f1,ru_m5 sample_rate=44100 state=ready"
        );
        assert!(ready.starts_with("READY"));
        assert!(ready.is_ascii());
        assert_eq!(Event::Started { id: 7 }.to_line(), "STARTED id=7");
        assert_eq!(Event::Playing { id: 7 }.to_line(), "PLAYING id=7");
        assert_eq!(Event::Done { id: 7 }.to_line(), "DONE id=7");
        assert_eq!(
            Event::Failed {
                id: 7,
                reason: "synth".into()
            }
            .to_line(),
            "FAILED id=7 reason=synth"
        );
        let rejected = Event::Rejected {
            reason: RejectReason::InvalidBase64,
        }
        .to_line();
        assert_eq!(rejected, "REJECTED reason=invalid-base64");
    }
}
