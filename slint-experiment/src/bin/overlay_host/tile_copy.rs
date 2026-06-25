//! 📋-copy / conversation-format LEAF helpers carved out of
//! `tile_controller.rs` (a later wave of the `overlay_host.rs` modularization —
//! see `docs/overlay-host-modularization-plan.md` §5.10 and
//! `docs/overlay-host-current-review.md` §"tile_controller.rs стал новым
//! мини-монолитом").
//!
//! This module owns the pure text-derivation behind the per-tile 📋 copy
//! button plus the follow-up directive plumbing, with their unit tests:
//!
//! - `message_text` — plain text of one chat message (text body, or the text
//!   Part(s) of a vision turn, NEVER the base64 image);
//! - `FOLLOWUP_DIRECTIVE` + `strip_followup_directives` — the marker prepended
//!   to a follow-up's user message (so a weak local model treats it as a DIRECT
//!   question, not transcript noise) and the helper that strips stale copies off
//!   prior turns;
//! - `user_question_for_copy` — peel the `build_request` wrapper off a user turn
//!   so the 📋 copy shows the real question, never the raw Mic/System dump;
//! - `convo_copy_text` / `format_convo_copy` — adaptive copy text (single answer
//!   vs whole labelled 🧑/🤖 thread); `convo_copy_text` reads the
//!   `OverlayBarBridge` conversation map (that bridge stays in
//!   `tile_controller.rs`, reached here through the crate-root glob);
//! - `wire_copy` — wire the 📋 button to write the answer to the Windows
//!   clipboard + flash ✅ (copy is purely local — no network egress, safe under
//!   screen-share / stealth);
//! - the `#[cfg(test)] mod copy_tests` exercising all of the above.
//!
//! SECURITY (unchanged by this move): copy never reaches the network, and the
//! transcript-stripping in `user_question_for_copy` keeps the raw Mic/System
//! lines out of the clipboard.
//!
//! NOTE (§7): this mechanical move imports the parent crate-root via
//! `use super::*;` (it reaches `ai::*`, `OverlayBarBridge`, the Slint
//! `TileWindow`, the clipboard helper, and the `vision` prompts through it).
//! That is intentional for the extraction; the imports get narrowed in a later
//! pass.
use super::{ai, vision, Arc, ComponentHandle, Duration, OverlayBarBridge, TileWindow, Timer};
// `conversations_evict_keys` lives in `tile_controller.rs`; only this module's
// eviction unit test (`copy_tests`) exercises it, so import it TEST-ONLY — a
// plain module-level import would be unused in the normal build (clippy -D).
#[cfg(test)]
use super::conversations_evict_keys;

/// Plain text of one chat message — the `Text` body, or for a vision turn the
/// concatenated text Part(s) only (NEVER the base64 image).
pub(crate) fn message_text(content: &ai::MessageContent) -> String {
    match content {
        ai::MessageContent::Text(t) => t.clone(),
        ai::MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| match p {
                ai::ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Build the clipboard text for the transcript "Copy all / Copy selected" (ТЗ1,
/// decision #7). One reply per line as `Спикер: текст`; with `with_timecodes`,
/// prefixed `[mm:ss] ` (session-relative, derived from `session_start_ms` — only
/// when it is `> 0`). When `selected` is `Some`, only those row indices are
/// included, in chronological (vector) order. Labels match the on-screen
/// transcript (decision #1: Система / Микрофон) and `build_session_markdown`;
/// internal whitespace is collapsed so one utterance = one line. Pure → tested.
///
/// Wired by the ТЗ1 transcript window's "Copy all" button
/// (`aux_windows::wire_transcript_copy`); the per-line "Copy selected" path will
/// pass a populated `selected` set in a later sub-increment.
pub(crate) fn format_transcript_for_copy(
    utts: &[overlay_backend::persistence::Utterance],
    session_start_ms: Option<i64>,
    selected: Option<&std::collections::HashSet<usize>>,
    with_timecodes: bool,
) -> String {
    let mut out = String::new();
    for (i, u) in utts.iter().enumerate() {
        if selected.is_some_and(|sel| !sel.contains(&i)) {
            continue;
        }
        let label = if u.source == "mic" {
            "Микрофон"
        } else {
            "Система"
        };
        let text = u.text.split_whitespace().collect::<Vec<_>>().join(" ");
        // F1: timecode = the line's START (previous line's timestamp; first = origin),
        // matching the on-screen transcript + the player seek.
        let prefix = if with_timecodes {
            overlay_backend::session_audio::line_start_offset_ms(utts, i, session_start_ms)
                .map(|off| format!("[{}] ", super::aux_windows::fmt_offset(off)))
                .unwrap_or_default()
        } else {
            String::new()
        };
        out.push_str(&format!("{prefix}{label}: {text}\n"));
    }
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Strip the `build_request` wrapper from a user turn for the 📋 copy, leaving
/// the actual question. The F9/auto ask bundles the live transcript as AI
/// context ("Транскрипт последних реплик…\n\nПомоги ответить: <q>"), so the real
/// question is the bit after "Помоги ответить:" — without that we'd copy the
/// raw Mic/System transcript lines into the chat copy. A transcript-only F9 ask
/// (no explicit question) → empty, so the noisy transcript is dropped; a typed
/// follow-up is already clean and passes through unchanged.
/// V0.8.3 — prepended to a follow-up's user message sent to the model. The
/// conversation's system prompt frames the assistant as "answer the last
/// question FROM THE TRANSCRIPT", so a bare follow-up was treated as transcript
/// noise and the model re-answered the original (user saw Sonnet reply "Два" to
/// "what is arc raider"). This marker makes the follow-up an explicit DIRECT
/// question. The UI + 📋 copy still show the clean question (it's stripped in
/// user_question_for_copy); the journal logs the raw question.
pub(crate) const FOLLOWUP_DIRECTIVE: &str =
    "Это прямой вопрос пользователя к тебе (НЕ из транскрипта, НЕ предыдущий вопрос). \
     Ответь именно на него: ";

pub(crate) fn user_question_for_copy(raw: &str) -> String {
    let raw = raw.strip_prefix(FOLLOWUP_DIRECTIVE).unwrap_or(raw);
    const MARK: &str = "Помоги ответить:";
    if let Some(i) = raw.rfind(MARK) {
        return raw[i + MARK.len()..].trim().to_string();
    }
    if raw.trim_start().starts_with("Транскрипт последних реплик") {
        return String::new();
    }
    // A vision tile's first user turn is the canned screenshot prompt, not text
    // the user typed — drop it so a multi-turn vision copy doesn't render
    // "🧑 Что на этом скриншоте?…" as if the user had asked it.
    if raw.trim() == vision::DEFAULT_VISION_PROMPT
        || raw.trim().starts_with(vision::TRANSLATE_VISION_PROMPT)
    {
        return String::new();
    }
    raw.trim().to_string()
}

/// Remove the [`FOLLOWUP_DIRECTIVE`] wrapper from the given user turns. Used when
/// building a follow-up / regenerate request so that only the CURRENT question
/// carries the directive. The wrapper is stored verbatim in `conversations`
/// (`handle_ai_event` Done folds `request_messages`), so without this a 3-turn
/// thread would send the model TWO "this is THE direct question" instructions on
/// two different historical turns — and a weak local model then anchors on the
/// wrong one. Non-user turns are left untouched.
pub(crate) fn strip_followup_directives(messages: &mut [ai::ChatMessage]) {
    for m in messages.iter_mut() {
        if m.role != "user" {
            continue;
        }
        let cleaned = match &m.content {
            ai::MessageContent::Text(t) => t.strip_prefix(FOLLOWUP_DIRECTIVE).map(str::to_string),
            _ => None,
        };
        if let Some(c) = cleaned {
            m.content = ai::MessageContent::Text(c);
        }
    }
}

/// V0.8.3 — text for the 📋 copy button. Adaptive so it fits both uses:
///
/// - a single Q→A tile → just the answer (clean paste — the "screenshot →
///   answer → paste it" case);
/// - a multi-turn dialog (a branch) → the WHOLE thread, every question +
///   answer, labelled 🧑 / 🤖 — so a conversation isn't truncated to its last
///   reply (user: "копируется только последнее сообщение, а не весь чат").
///
/// System prompts are skipped; vision turns contribute their text only. Empty
/// if the tile has no (or an unknown / not-yet-seeded) conversation.
pub(crate) fn convo_copy_text(bridge: &OverlayBarBridge, convo_id: i32) -> String {
    let convos = bridge
        .conversations
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    match convos.get(&convo_id) {
        Some(c) => format_convo_copy(&c.messages, &c.rendered),
        None => String::new(),
    }
}

/// Pure formatter behind [`convo_copy_text`] — split out (no bridge / no lock)
/// so the adaptive single-vs-thread logic and the user-turn cleaning are
/// unit-testable. `rendered` is the mid-stream fallback (used when there is no
/// recorded assistant turn yet, or when every turn cleans to empty).
pub(crate) fn format_convo_copy(messages: &[ai::ChatMessage], rendered: &str) -> String {
    let turns: Vec<(&str, String)> = messages
        .iter()
        .filter(|m| m.role != "system")
        .filter_map(|m| {
            let t = message_text(&m.content).trim().to_string();
            (!t.is_empty()).then_some((m.role.as_str(), t))
        })
        .collect();
    if turns.is_empty() {
        return rendered.to_string();
    }
    let assistant_turns = turns.iter().filter(|(r, _)| *r == "assistant").count();
    if assistant_turns <= 1 {
        // Single answer: copy just it (or the rendered body if, mid-stream, no
        // assistant turn is recorded yet).
        return turns
            .iter()
            .rev()
            .find(|(r, _)| *r == "assistant")
            .map(|(_, t)| t.clone())
            .unwrap_or_else(|| rendered.to_string());
    }
    let mut out = String::new();
    for (role, text) in &turns {
        // User turns carry the build_request wrapper (transcript + "Помоги
        // ответить:") — copy only the real question, never the Mic/System dump.
        let display = if *role == "assistant" {
            (*text).clone()
        } else {
            user_question_for_copy(text)
        };
        if display.trim().is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(if *role == "assistant" {
            "Assistant: "
        } else {
            "You: "
        });
        out.push_str(display.trim());
    }
    if out.is_empty() {
        return rendered.to_string();
    }
    out
}

/// V0.8.3 — wire a tile's copy button: write the answer text to the Windows
/// clipboard and flash feedback for ~1.5 s. Called for every
/// conversational tile (those with a `convo_id`). Copy is purely local — no
/// network egress — so it stays safe under screen-share / stealth.
pub(crate) fn wire_copy(tile: &TileWindow, convo_id: i32, bridge: &Arc<OverlayBarBridge>) {
    tile.set_can_copy(true);
    let weak = tile.as_weak();
    let bridge_c = bridge.clone();
    tile.on_copy_clicked(move || {
        let text = convo_copy_text(&bridge_c, convo_id);
        if text.is_empty() {
            return;
        }
        match clipboard_win::set_clipboard_string(&text) {
            Ok(()) => {
                let Some(t) = weak.upgrade() else {
                    return;
                };
                t.set_copied(true);
                let w = t.as_weak();
                Timer::single_shot(Duration::from_millis(1500), move || {
                    if let Some(t) = w.upgrade() {
                        t.set_copied(false);
                    }
                });
            }
            Err(e) => eprintln!("[overlay-host] clipboard copy failed: {e}"),
        }
    });
}

/// Text for the 🔊 read-aloud: the LATEST assistant answer only — never the user
/// prompts / transcript / earlier turns. (The 📋 copy deliberately includes the
/// whole labelled thread; read-aloud must NOT, or it speaks your own questions
/// back at you — the bug the tester hit.) Falls back to the rendered body
/// mid-stream, before an assistant turn is recorded.
pub(crate) fn convo_speak_text(bridge: &OverlayBarBridge, convo_id: i32) -> String {
    let convos = bridge
        .conversations
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    match convos.get(&convo_id) {
        Some(c) => speak_answer_text(&c.messages, &c.rendered),
        None => String::new(),
    }
}

/// Pure: the latest assistant turn's text, or the rendered body if none yet.
pub(crate) fn speak_answer_text(messages: &[ai::ChatMessage], rendered: &str) -> String {
    messages
        .iter()
        .rev()
        .find(|m| m.role == "assistant")
        .map(|m| message_text(&m.content).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| rendered.trim().to_string())
}

// Which tile is currently being read aloud. TTS is process-global +
// one-utterance-at-a-time, so we remember the convo_id that started the current
// speech: closing THAT tile (or the app) stops it; a new speak re-points it.
static SPEAKING_CONVO: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(i64::MIN);

/// Record that `convo_id` started the current read-aloud.
pub(crate) fn mark_speaking(convo_id: i32) {
    SPEAKING_CONVO.store(convo_id as i64, std::sync::atomic::Ordering::Release);
}

/// Stop the read-aloud iff `convo_id` is the tile currently being spoken — called
/// from each tile's close handler so closing the speaking tile silences it.
pub(crate) fn stop_if_speaking(convo_id: i32) {
    if SPEAKING_CONVO.load(std::sync::atomic::Ordering::Acquire) == convo_id as i64 {
        overlay_backend::tts::stop();
        SPEAKING_CONVO.store(i64::MIN, std::sync::atomic::Ordering::Release);
    }
}

/// The convo_id currently being read aloud, or -1 if none.
pub(crate) fn current_speaking_convo() -> i32 {
    let v = SPEAKING_CONVO.load(std::sync::atomic::Ordering::Acquire);
    if v == i64::MIN {
        -1
    } else {
        v as i32
    }
}

// Process-global pause latch, shared by the tile ⏯ button AND the Shift+Alt+3
// hotkey so they stay coherent (TTS is global + one-at-a-time). false = playing.
static SPEAK_PAUSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Toggle pause/resume of the current read-aloud; returns the NEW paused state.
pub(crate) fn toggle_pause() -> bool {
    let now_paused = !SPEAK_PAUSED.fetch_xor(true, std::sync::atomic::Ordering::SeqCst);
    if now_paused {
        overlay_backend::tts::pause();
    } else {
        overlay_backend::tts::resume();
    }
    now_paused
}

/// Reset the latch to "playing" — called when a fresh utterance starts.
pub(crate) fn reset_pause() {
    SPEAK_PAUSED.store(false, std::sync::atomic::Ordering::SeqCst);
}

/// Read-aloud — wire a tile's 🔊 «Озвучить» + ⏯ pause/resume controls to the
/// process-global neural TTS sidecar. Speaks ONLY the latest answer (never the
/// prompts / earlier turns). Purely local — no network egress — so it stays safe
/// under screen-share / stealth.
pub(crate) fn wire_speak(tile: &TileWindow, convo_id: i32, bridge: &Arc<OverlayBarBridge>) {
    tile.set_can_speak(true);
    let bridge_speak = bridge.clone();
    {
        let weak = tile.as_weak();
        tile.on_speak_clicked(move || {
            let text = convo_speak_text(&bridge_speak, convo_id);
            if text.trim().is_empty() {
                return;
            }
            reset_pause();
            if let Some(t) = weak.upgrade() {
                t.set_speak_paused(false);
            }
            mark_speaking(convo_id);
            overlay_backend::tts::speak(&text);
        });
    }
    let weak_p = tile.as_weak();
    tile.on_speak_pause_clicked(move || {
        // Shared global latch so this ⏯ button and the Shift+Alt+3 hotkey stay
        // coherent (one TTS engine, one utterance at a time).
        let now_paused = toggle_pause();
        if let Some(t) = weak_p.upgrade() {
            t.set_speak_paused(now_paused);
        }
    });
}

#[cfg(test)]
mod copy_tests {
    //! Locks the 📋-copy text derivation — the exact area the user hit live:
    //! copy pulling in the raw Mic/System transcript, and follow-ups being
    //! re-answered as the original question. Pure: no bridge, no UI, no network.
    use super::*;

    #[test]
    fn transcript_copy_format() {
        use overlay_backend::persistence::Utterance;
        let start = 1000_i64;
        let utts = vec![
            Utterance {
                session_id: "s".into(),
                unix_ms: start + 29_000, // finalized ~00:29 into the session (≈ its end)
                source: "system".into(),
                text: "привет  мир".into(), // double space collapses
                audio_ms: None,
            },
            Utterance {
                session_id: "s".into(),
                unix_ms: start + 135_000,
                source: "mic".into(),
                text: "да".into(),
                audio_ms: None,
            },
        ];
        // Default: "Спикер: текст", no timecodes, all lines, no trailing newline.
        assert_eq!(
            format_transcript_for_copy(&utts, Some(start), None, false),
            "Система: привет мир\nМикрофон: да"
        );
        // With timecodes — F1: a line's START = the PREVIOUS line's timestamp; the
        // FIRST line is 00:00 (NOT its own finalize time 00:29), so line 2 starts
        // where line 1 ended (00:29).
        assert_eq!(
            format_transcript_for_copy(&utts, Some(start), None, true),
            "[00:00] Система: привет мир\n[00:29] Микрофон: да"
        );
        // Selected subset (only row 1), chronological order.
        let mut sel = std::collections::HashSet::new();
        sel.insert(1_usize);
        assert_eq!(
            format_transcript_for_copy(&utts, Some(start), Some(&sel), false),
            "Микрофон: да"
        );
        // Empty transcript → empty string.
        assert_eq!(
            format_transcript_for_copy(&[], Some(start), None, false),
            ""
        );
        // with_timecodes but no session start → no prefix.
        assert_eq!(
            format_transcript_for_copy(&utts[..1], None, None, true),
            "Система: привет мир"
        );
    }

    fn msg(role: &str, text: &str) -> ai::ChatMessage {
        ai::ChatMessage {
            role: role.to_string(),
            content: ai::MessageContent::Text(text.to_string()),
        }
    }
    fn parts_msg(role: &str, texts: &[&str]) -> ai::ChatMessage {
        ai::ChatMessage {
            role: role.to_string(),
            content: ai::MessageContent::Parts(
                texts
                    .iter()
                    .map(|t| ai::ContentPart::Text {
                        text: (*t).to_string(),
                    })
                    .collect(),
            ),
        }
    }

    #[test]
    fn message_text_text_and_parts() {
        assert_eq!(
            message_text(&ai::MessageContent::Text("plain".into())),
            "plain"
        );
        // Parts: text parts are joined (image parts, when present, contribute
        // nothing — exercised here with two text parts).
        let m = parts_msg("user", &["hello", "world"]);
        assert_eq!(message_text(&m.content), "hello\nworld");
    }

    #[test]
    fn copy_question_strips_transcript_wrapper() {
        let raw = "Транскрипт последних реплик:\n[СОБЕСЕДНИК] arc raiders?\n\n\
                   Помоги ответить: что такое arc raiders";
        assert_eq!(user_question_for_copy(raw), "что такое arc raiders");
    }

    #[test]
    fn conversations_evict_keys_drops_oldest_half_keeps_newest() {
        // FIX #8 — at the cap, the lowest-id half (oldest tiles) is evicted,
        // and the highest ids (newest / currently-open tiles) are kept.
        let keys: Vec<i32> = (0..256).collect();
        let evicted = conversations_evict_keys(&keys, 256);
        assert_eq!(evicted.len(), 128, "evicts exactly half the cap");
        assert_eq!(evicted.first(), Some(&0), "oldest id is evicted");
        assert_eq!(evicted.last(), Some(&127), "eviction stops at the midpoint");
        assert!(
            !evicted.contains(&255),
            "the newest id (an open tile) is never evicted"
        );
        // Unsorted input is handled (HashMap key order is arbitrary).
        let shuffled = [50, 3, 200, 7, 99];
        let mut e = conversations_evict_keys(&shuffled, 4); // max/2 = 2 → drop 2 lowest
        e.sort_unstable();
        assert_eq!(
            e,
            vec![3, 7],
            "drops the two lowest ids regardless of order"
        );
    }

    #[test]
    fn copy_question_drops_transcript_only_ask() {
        let raw = "Транскрипт последних реплик:\n[СОБЕСЕДНИК] что-то сказал";
        assert_eq!(user_question_for_copy(raw), "");
    }

    #[test]
    fn copy_question_strips_followup_directive() {
        let raw = format!("{FOLLOWUP_DIRECTIVE}а что дальше?");
        assert_eq!(user_question_for_copy(&raw), "а что дальше?");
    }

    #[test]
    fn copy_question_drops_canned_vision_prompt() {
        assert_eq!(user_question_for_copy(vision::DEFAULT_VISION_PROMPT), "");
    }

    #[test]
    fn copy_question_drops_translate_vision_prompt() {
        // Feature #3 — a translate tile's first turn is the canned translate
        // prompt, not user-typed text → drop it (both phonetics states; the ON
        // variant is base+suffix, so starts_with the base still matches).
        assert_eq!(user_question_for_copy(vision::TRANSLATE_VISION_PROMPT), "");
        assert_eq!(user_question_for_copy(&vision::translate_prompt(true)), "");
    }

    #[test]
    fn copy_question_passes_plain_text_trimmed() {
        assert_eq!(user_question_for_copy("  привет  "), "привет");
    }

    #[test]
    fn single_turn_copies_only_the_answer() {
        let msgs = vec![
            msg("system", "ты ассистент"),
            msg("user", "Помоги ответить: что такое Rust"),
            msg("assistant", "Rust — системный язык."),
        ];
        assert_eq!(
            format_convo_copy(&msgs, "RENDERED"),
            "Rust — системный язык."
        );
    }

    #[test]
    fn multi_turn_copies_labelled_thread_without_transcript() {
        let msgs = vec![
            msg("system", "ты ассистент"),
            msg(
                "user",
                "Транскрипт последних реплик:\n[СОБЕСЕДНИК] x\n\nПомоги ответить: вопрос 1",
            ),
            msg("assistant", "ответ 1"),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}вопрос 2")),
            msg("assistant", "ответ 2"),
        ];
        let out = format_convo_copy(&msgs, "RENDERED");
        assert_eq!(
            out,
            "You: вопрос 1\n\nAssistant: ответ 1\n\nYou: вопрос 2\n\nAssistant: ответ 2"
        );
        // The raw Mic/System transcript must never reach the clipboard.
        assert!(!out.contains("СОБЕСЕДНИК"));
    }

    #[test]
    fn multi_turn_vision_skips_canned_prompt() {
        let msgs = vec![
            parts_msg("user", &[vision::DEFAULT_VISION_PROMPT]),
            msg("assistant", "на экране код"),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}а на каком языке?")),
            msg("assistant", "на Rust"),
        ];
        let out = format_convo_copy(&msgs, "RENDERED");
        assert_eq!(
            out,
            "Assistant: на экране код\n\nYou: а на каком языке?\n\nAssistant: на Rust"
        );
    }

    #[test]
    fn empty_conversation_falls_back_to_rendered() {
        assert_eq!(format_convo_copy(&[], "RENDERED"), "RENDERED");
    }

    #[test]
    fn speak_reads_latest_answer_only_not_prompts_or_old_turns() {
        // The tester bug: 🔊 on a multi-turn tile read the prompts + every
        // message. Read-aloud must speak ONLY the latest answer.
        let msgs = vec![
            msg("system", "ты ассистент"),
            msg("user", "Помоги ответить: вопрос 1"),
            msg("assistant", "ответ один"),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}вопрос 2")),
            msg("assistant", "ответ два"),
        ];
        let spoken = speak_answer_text(&msgs, "RENDERED");
        assert_eq!(spoken, "ответ два");
        assert!(!spoken.contains("вопрос"));
        assert!(!spoken.contains("ответ один"));
    }

    #[test]
    fn speak_falls_back_to_rendered_before_any_answer() {
        let msgs = vec![msg("user", "Помоги ответить: q")];
        assert_eq!(
            speak_answer_text(&msgs, "частичный ответ"),
            "частичный ответ"
        );
        assert_eq!(speak_answer_text(&[], "RENDERED"), "RENDERED");
    }

    #[test]
    fn strip_directives_cleans_user_turns_only() {
        let mut msgs = [
            msg("system", &format!("{FOLLOWUP_DIRECTIVE}sys")),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}вопрос")),
            msg("assistant", &format!("{FOLLOWUP_DIRECTIVE}ответ")),
            msg("user", "уже чистый"),
        ];
        strip_followup_directives(&mut msgs);
        // system + assistant turns are untouched (only user turns get cleaned).
        assert_eq!(
            message_text(&msgs[0].content),
            format!("{FOLLOWUP_DIRECTIVE}sys")
        );
        assert_eq!(
            message_text(&msgs[2].content),
            format!("{FOLLOWUP_DIRECTIVE}ответ")
        );
        // user turns are stripped; an already-clean one is unchanged.
        assert_eq!(message_text(&msgs[1].content), "вопрос");
        assert_eq!(message_text(&msgs[3].content), "уже чистый");
    }

    #[test]
    fn strip_all_but_last_preserves_reasked_turn() {
        // Mirrors fire_regenerate's `&mut messages[..len-1]`: prior turns are
        // cleaned, but the last (re-asked) turn keeps whatever framing it had.
        let mut msgs = [
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}старый вопрос")),
            msg("assistant", "старый ответ"),
            msg(
                "user",
                &format!("{FOLLOWUP_DIRECTIVE}перезапрашиваемый вопрос"),
            ),
        ];
        let n = msgs.len() - 1;
        strip_followup_directives(&mut msgs[..n]);
        // Prior user turn is cleaned…
        assert_eq!(message_text(&msgs[0].content), "старый вопрос");
        // …but the last (re-asked) turn keeps its direct-question framing.
        assert_eq!(
            message_text(&msgs[2].content),
            format!("{FOLLOWUP_DIRECTIVE}перезапрашиваемый вопрос")
        );
    }
}
