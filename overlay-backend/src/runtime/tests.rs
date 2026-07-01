//! Unit tests for `runtime.rs`, split out to keep the module file lean.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::events::Noop;

/// Build a hermetic empty SharedConfig that does NOT load the
/// user's real `%APPDATA%\overlay-mvp\config.json` (which on the
/// dev machine contains live `ai_bearer` + `groq_api_key`).
/// Calling `crate::config::shared()` directly would read those
/// real secrets into the test process AND make the test hit
/// the real Anthropic endpoint — both unacceptable for unit
/// tests. Use this helper for any test that constructs a
/// `SharedConfig` for an async port body.
fn hermetic_empty_config() -> crate::config::SharedConfig {
    use parking_lot::RwLock;
    Arc::new(RwLock::new(crate::config::Config::default()))
}

/// Smoke test that the debrief port compiles + runs with Noop
/// events sink. With an explicitly empty AI config the call
/// short-circuits on the AI error path (no tile spawned); we
/// verify the fn doesn't panic + returns.
#[tokio::test]
async fn run_post_meeting_debrief_with_noop_events_does_not_panic() {
    let cfg = hermetic_empty_config();
    let transcript = vec![TranscriptLine {
        source: AudioSource::Mic,
        text: "test utterance".into(),
        timestamp_ms: 0,
    }];
    let sink: Arc<dyn RuntimeEvents> = Arc::new(Noop);
    // Fire-and-forget — with empty ai_bearer the AI call fails
    // and the fn returns without spawning a tile. Either way
    // no panic, no resource leak.
    run_post_meeting_debrief(sink, cfg, transcript, String::new()).await;
}

// ── Meeting-summary battery (v0.12.0 — S1) ──

fn line(source: AudioSource, text: &str, ms: u64) -> TranscriptLine {
    TranscriptLine {
        source,
        text: text.into(),
        timestamp_ms: ms,
    }
}

#[test]
fn summary_seed_truncates_over_local_budget_and_flags_system() {
    // A transcript well over the 12k local budget → user turn carries the
    // middle marker, system gains the «усечён» note. ~250 chars × 80 = 20k.
    let big: Vec<TranscriptLine> = (0..80)
        .map(|i| {
            line(
                AudioSource::System,
                &format!("реплика {i} {}", "слово ".repeat(40)),
                i,
            )
        })
        .collect();
    let seed = build_summary_seed(&big, true, true, None);
    let sys = match &seed[0].content {
        ai::MessageContent::Text(s) => s.clone(),
        _ => String::new(),
    };
    let usr = match &seed[1].content {
        ai::MessageContent::Text(s) => s.clone(),
        _ => String::new(),
    };
    assert!(sys.contains("усечён"), "truncated system must warn");
    assert!(
        usr.contains("пропущена"),
        "user turn must carry middle marker"
    );
    assert!(
        usr.chars().count() <= 12_000 + 200,
        "stays within budget + marker"
    );
}

#[test]
fn summary_gate_requires_two_lines() {
    assert!(summary_gate(&[]).is_err());
    assert!(summary_gate(&[line(AudioSource::Mic, "привет", 0)]).is_err());
    assert!(summary_gate(&[
        line(AudioSource::Mic, "привет", 0),
        line(AudioSource::System, "здравствуйте", 1),
    ])
    .is_ok());
}

#[test]
fn summary_format_labels_channels_ru_en() {
    let t = vec![
        line(AudioSource::Mic, " моя реплика ", 0),
        line(AudioSource::System, "их реплика", 1),
        line(AudioSource::Mic, "   ", 2), // whitespace-only — dropped
    ];
    assert_eq!(
        format_transcript_for_summary(&t, true),
        "Вы: моя реплика\nСобеседник: их реплика"
    );
    assert_eq!(
        format_transcript_for_summary(&t, false),
        "You: моя реплика\nInterlocutor: их реплика"
    );
}

#[test]
fn summary_truncate_passes_under_budget_unchanged() {
    let text = "Вы: раз\nСобеседник: два";
    let (out, truncated) = truncate_transcript_middle(text, 1_000, true);
    assert_eq!(out, text);
    assert!(!truncated);
}

#[test]
fn summary_truncate_keeps_head_tail_and_marker() {
    // 20 lines × 10 chars (incl. newline cost) — budget 100 keeps
    // ~3 head lines + ~6 tail lines, drops the middle.
    let lines: Vec<String> = (0..20).map(|i| format!("Вы: ст{i:03}")).collect();
    let text = lines.join("\n");
    let (out, truncated) = truncate_transcript_middle(&text, 100, true);
    assert!(truncated);
    assert!(out.contains("пропущена"), "marker missing: {out}");
    assert!(out.starts_with("Вы: ст000"), "head must survive: {out}");
    assert!(out.ends_with("Вы: ст019"), "tail must survive: {out}");
    assert!(!out.contains("ст010"), "middle must be dropped: {out}");
}

#[test]
fn summary_truncate_handles_single_giant_line() {
    // No newlines at all — line-based cut degenerates; the char-slice
    // fallback must still deliver head + marker + tail.
    let text = "а".repeat(500);
    let (out, truncated) = truncate_transcript_middle(&text, 90, true);
    assert!(truncated);
    assert!(out.contains("пропущена"));
    assert!(out.starts_with(&"а".repeat(30)));
    assert!(out.ends_with(&"а".repeat(60)));
    assert!(out.chars().count() < 500);
}

#[test]
fn summary_prompt_has_sections_and_honesty_rules_ru_en() {
    let ru = summary_system_prompt(true, false);
    for s in [
        "Участники",
        "О чём говорили",
        "Решения",
        "Задачи",
        "Договорённости",
    ] {
        assert!(ru.contains(s), "ru prompt missing section {s}");
    }
    assert!(ru.contains("НЕ выдумывай"));
    assert!(ru.contains("(неточно)"));
    assert!(ru.contains("НЕСКОЛЬКО"));
    assert!(!ru.contains("усечён"));
    assert!(summary_system_prompt(true, true).contains("усечён"));

    let en = summary_system_prompt(false, false);
    for s in [
        "Participants",
        "Topics discussed",
        "Decisions",
        "Action items",
        "Agreements",
    ] {
        assert!(en.contains(s), "en prompt missing section {s}");
    }
    assert!(en.contains("do NOT invent"));
    assert!(en.contains("(uncertain)"));
    assert!(!en.contains("cut in the middle"));
    assert!(summary_system_prompt(false, true).contains("cut in the middle"));
}

#[test]
fn summary_seed_is_system_plus_user_with_transcript() {
    let t = vec![
        line(AudioSource::Mic, "обсудим план", 0),
        line(AudioSource::System, "давай, я записываю", 1),
    ];
    let seed = build_summary_seed(&t, true, false, None);
    assert_eq!(seed.len(), 2, "seed must be exactly [system, user]");
    assert_eq!(seed[0].role, "system");
    assert_eq!(seed[1].role, "user");
    // System carries the recap instructions…
    let sys = match &seed[0].content {
        ai::MessageContent::Text(s) => s.clone(),
        _ => String::new(),
    };
    assert!(sys.contains("Участники"));
    // …and the user turn is the channel-labelled transcript (NOT a title),
    // so a 1-user-turn regenerate re-asks THIS and rebuilds the summary.
    let usr = match &seed[1].content {
        ai::MessageContent::Text(s) => s.clone(),
        _ => String::new(),
    };
    assert_eq!(usr, "Вы: обсудим план\nСобеседник: давай, я записываю");
}

#[test]
fn summary_seed_matches_what_run_meeting_summary_would_send() {
    // The seed used by the tile must equal the bar-button's request pair so
    // 🔄/🧠 rebuild byte-identically. Local budget path (12k) over a short
    // transcript = no truncation, so the system has no "усечён" note.
    let t = vec![
        line(AudioSource::Mic, "коротко", 0),
        line(AudioSource::System, "ок", 1),
    ];
    let seed = build_summary_seed(&t, true, true, None);
    let sys = match &seed[0].content {
        ai::MessageContent::Text(s) => s.clone(),
        _ => String::new(),
    };
    assert!(!sys.contains("усечён"));
    assert_eq!(sys, summary_system_prompt(true, false));
}

// ── v0.17.0 map-reduce (план B: 7-8 h workdays) ──

#[test]
fn split_for_map_packs_lines_within_budget_and_preserves_words() {
    let formatted = "Вы: один два три\nСобеседник: четыре пять\nВы: шесть семь восемь";
    let parts = split_transcript_for_map(formatted, 30);
    assert!(parts.len() >= 2, "{parts:?}");
    for p in &parts {
        assert!(p.chars().count() <= 30, "part over budget: {p:?}");
    }
    // No words lost or reordered.
    let joined: Vec<&str> = parts.iter().flat_map(|p| p.split_whitespace()).collect();
    let original: Vec<&str> = formatted.split_whitespace().collect();
    assert_eq!(joined, original);
}

#[test]
fn split_for_map_word_wraps_one_giant_line() {
    // The re-Summary transcript is ONE giant line per channel — exactly
    // план B's case. A single line over budget must word-wrap, not become
    // one oversized part.
    let giant = format!("Вы: {}", "слово ".repeat(200).trim_end());
    let parts = split_transcript_for_map(&giant, 100);
    assert!(parts.len() > 5, "{}", parts.len());
    for p in &parts {
        assert!(p.chars().count() <= 100, "part over budget");
    }
    let joined: Vec<&str> = parts.iter().flat_map(|p| p.split_whitespace()).collect();
    let original: Vec<&str> = giant.split_whitespace().collect();
    assert_eq!(joined, original);
}

#[test]
fn reduce_seed_bounds_combined_conspectuses_to_the_provider_budget() {
    // 12 parts × ~2.8k chars = ~34k > both budgets → the reduce input must
    // be middle-truncated to the provider budget (else the local
    // llama-server's -c 8192 overflows) and the system gains the
    // truncation note.
    let partials: Vec<String> = (0..12)
        .map(|i| format!("- факт {i} {}", "x".repeat(2800)))
        .collect();
    let seed = build_summary_reduce_seed(&partials, true, true, None);
    let usr = match &seed[1].content {
        ai::MessageContent::Text(s) => s.clone(),
        _ => String::new(),
    };
    assert!(
        usr.chars().count() <= SUMMARY_INPUT_BUDGET_LOCAL_CHARS + 200,
        "reduce input over local budget: {}",
        usr.chars().count()
    );
    let sys = match &seed[0].content {
        ai::MessageContent::Text(s) => s.clone(),
        _ => String::new(),
    };
    assert!(sys.contains("усечён"), "truncation note must be flagged");
}

#[test]
fn reduce_seed_carries_rules_part_headers_and_memory_ref() {
    let partials = vec!["- тема А".to_string(), "- тема Б".to_string()];
    let seed = build_summary_reduce_seed(&partials, true, false, Some("- Альфа — CRM"));
    assert_eq!(seed.len(), 2);
    let sys = match &seed[0].content {
        ai::MessageContent::Text(s) => s.clone(),
        _ => String::new(),
    };
    // Final pass keeps the five-section rules + gains the reduce note +
    // the decode-only СПРАВКА.
    assert!(sys.contains("Участники"));
    assert!(sys.contains("КОНСПЕКТЫ ПОСЛЕДОВАТЕЛЬНЫХ"));
    assert!(sys.contains("СПРАВКА"));
    assert!(sys.contains("Альфа — CRM"));
    let usr = match &seed[1].content {
        ai::MessageContent::Text(s) => s.clone(),
        _ => String::new(),
    };
    assert!(usr.contains("=== Часть 1/2 ==="));
    assert!(usr.contains("=== Часть 2/2 ==="));
    assert!(usr.contains("- тема А"));
    assert!(usr.contains("- тема Б"));
}

#[test]
fn partial_prompt_is_no_fabrication_and_part_numbered() {
    let ru = partial_summary_prompt(true, 3, 7);
    assert!(ru.contains("ЧАСТЬ 3/7"));
    assert!(ru.contains("НЕ"));
    assert!(ru.contains("(неточно)"));
    let en = partial_summary_prompt(false, 1, 2);
    assert!(en.contains("PART 1/2"));
    assert!(en.contains("do NOT invent"));
}

#[test]
fn summary_seed_memory_ref_is_decode_only_and_none_is_byte_identical() {
    fn text_of(m: &ai::ChatMessage) -> String {
        match &m.content {
            ai::MessageContent::Text(s) => s.clone(),
            _ => String::new(),
        }
    }
    let t = vec![
        line(AudioSource::Mic, "обсудим по Альфе", 0),
        line(AudioSource::System, "давай", 1),
    ];
    // None / empty / whitespace → byte-identical to the pre-v0.16 seed.
    let plain = build_summary_seed(&t, true, false, None);
    let empty = build_summary_seed(&t, true, false, Some("   "));
    assert_eq!(text_of(&plain[0]), text_of(&empty[0]));
    assert_eq!(text_of(&plain[1]), text_of(&empty[1]));
    // Some(block) → the system prompt gains the decode-only СПРАВКА with
    // the block, and the user turn (transcript) is untouched.
    let with_ref = build_summary_seed(&t, true, false, Some("- Проект Альфа — внутренняя CRM"));
    let sys = text_of(&with_ref[0]);
    assert!(sys.contains("СПРАВКА"));
    assert!(sys.contains("Проект Альфа — внутренняя CRM"));
    assert!(sys.contains("НЕ добавляй из справки факты"));
    assert_eq!(
        text_of(&with_ref[1]),
        text_of(&plain[1]),
        "user turn must be unchanged"
    );
}

/// Smoke: with a hermetic empty config the AI call fails fast and the
/// fn takes the generic-ERROR-tile branch (button feedback) without
/// panicking and without touching the network (empty base_url).
#[tokio::test]
async fn run_meeting_summary_with_noop_events_does_not_panic() {
    let cfg = hermetic_empty_config();
    let transcript = vec![
        line(AudioSource::Mic, "обсуждаем план", 0),
        line(AudioSource::System, "согласен, делаем", 1),
    ];
    let sink: Arc<dyn RuntimeEvents> = Arc::new(Noop);
    // Empty session id = the "ephemeral / don't persist" sentinel, so the
    // conspect sidecar is never touched and the test stays hermetic (no
    // write to the real %APPDATA%).
    run_meeting_summary(sink, cfg, transcript, String::new(), false).await;
}

/// v0.18.6 invariant: a conspect carries the part SUMMARIES the reduce needs,
/// and the resumable pipeline reduces ONLY over non-empty summaries — it must
/// never send the model a reduce whose parts are blank (that is exactly what
/// made the model beg "предоставьте текст конспектов части 1/3…"). This pins
/// both the filtering and that a real reduce seed embeds the part text.
#[test]
fn reduce_only_runs_over_non_empty_part_summaries() {
    let mut cs = Conspect::new(
        "sess".into(),
        true,
        conspect::fingerprint("t"),
        false,
        vec!["src a".into(), "src b".into(), "src c".into()],
    );
    cs.parts[0].summary = Some("- решили выкатить в пятницу".into());
    cs.parts[1].summary = None; // this part's map failed
    cs.parts[2].summary = Some("   ".into()); // blank → not usable
    let summaries = cs.usable_summaries();
    assert_eq!(
        summaries,
        vec!["- решили выкатить в пятницу".to_string()],
        "only the real conspectus survives"
    );
    // The reduce seed built from it actually carries the part text — so the
    // model is never handed an empty input that it would answer by asking
    // for the conspect.
    let seed = build_summary_reduce_seed(&summaries, true, true, None);
    let user = match &seed[1].content {
        ai::MessageContent::Text(t) => t.clone(),
        ai::MessageContent::Parts(_) => String::new(),
    };
    assert!(user.contains("решили выкатить в пятницу"));
    // And the missing-part bookkeeping points the retry at the failed slice.
    assert_eq!(cs.missing_part_indices(), vec![1, 2]);
}

// ── Prompt-builder battery (moved from src-tauri Phase B2 port #2) ──
// These tests don't call AI — they exercise build_auto_tile_prompts
// with adversarial / edge-case inputs and assert the resulting
// prompt STILL contains the safety + formatting rules. Catches
// regressions where someone shortens the prompt and accidentally
// drops a guard.

/// Anti-prompt-injection block must always appear, regardless of input
/// shape — it's the only thing defending the model from interviewer
/// transcript containing "ignore previous instructions" style payloads.
#[test]
fn prompt_always_contains_injection_guard() {
    for (lines, ctx) in &[
        (vec![], ""),
        (vec!["normal line".to_string()], "Senior SRE"),
        (vec!["a".to_string(); 50], "x".repeat(2000).as_str()),
    ] {
        let (sys, _usr) =
            build_auto_tile_prompts(&Trigger::Question("q".into()), lines, ctx, "ru", false);
        assert!(
            sys.contains("БЕЗОПАСНОСТЬ"),
            "system prompt missing anti-injection block for input shape {lines:?}"
        );
        assert!(
            sys.contains("забудь предыдущие инструкции") || sys.contains("игнорируй"),
            "system prompt missing injection-defense wording"
        );
    }
}

/// Garbage / off-topic guard must appear — without it, the model
/// makes up answers to malformed transcripts.
#[test]
fn prompt_contains_garbage_and_offtopic_guards() {
    let (sys, _) = build_auto_tile_prompts(&Trigger::Question("test".into()), &[], "", "ru", false);
    assert!(sys.contains("мусор"), "missing garbage-input rule");
    assert!(sys.contains("повтори?"), "missing 'повтори?' fallback");
    assert!(
        sys.contains("не про техническую"),
        "missing off-topic short-circuit"
    );
    assert!(
        sys.contains("НЕ ВЫДУМЫВАЙ"),
        "missing 'don't fabricate' rule"
    );
}

/// Whisper artifact recovery hints must be in the prompt — these are
/// the canonical Cyrillic-mangling → Latin recoveries.
#[test]
fn prompt_contains_whisper_artifact_recovery_hints() {
    let (sys, _) = build_auto_tile_prompts(&Trigger::Question("test".into()), &[], "", "ru", false);
    assert!(sys.contains("К87С") || sys.contains("K8s"));
    assert!(sys.contains("гинкс") || sys.contains("nginx"));
    // Newly added in morning addendum:
    assert!(sys.contains("3к") || sys.contains("k3s"));
    assert!(sys.contains("эстиди") || sys.contains("etcd"));
    assert!(sys.contains("истио") || sys.contains("istio"));
}

/// Long transcript (50 lines) must still produce a usable user prompt
/// (not panic, includes the trigger text + recent context).
#[test]
fn prompt_handles_long_transcript() {
    let lines: Vec<String> = (0..50)
        .map(|i| format!("Это реплика номер {i} с упоминанием Kubernetes."))
        .collect();
    let (_sys, usr) = build_auto_tile_prompts(
        &Trigger::Question("Что такое kubernetes?".into()),
        &lines,
        "Senior SRE interview, 7 years k8s",
        "ru",
        false,
    );
    assert!(usr.contains("Что такое kubernetes?"));
    assert!(
        usr.contains("реплика номер 49"),
        "missing newest transcript lines"
    );
}

/// Empty transcript must not crash + still produce coherent prompt.
#[test]
fn prompt_handles_empty_transcript() {
    let (sys, usr) = build_auto_tile_prompts(&Trigger::Question("q?".into()), &[], "", "ru", false);
    assert!(!sys.is_empty());
    assert!(!usr.is_empty());
    assert!(
        usr.contains("транскрипт пуст") || usr.contains("(транскрипт пуст)"),
        "empty-transcript placeholder missing"
    );
}

/// Russian language rule must dominate when response_language="ru".
#[test]
fn prompt_enforces_russian_response_when_configured() {
    let (sys, _) = build_auto_tile_prompts(
        &Trigger::Question("how to scale?".into()),
        &[],
        "",
        "ru",
        false,
    );
    assert!(
        sys.contains("ИСКЛЮЧИТЕЛЬНО на русском"),
        "missing strict Russian rule"
    );
}

/// Off-topic question handler is still present even when meeting context
/// is empty (no SRE prior to anchor against).
#[test]
fn prompt_offtopic_guard_present_with_empty_context() {
    let (sys, _) = build_auto_tile_prompts(
        &Trigger::Question("Какая погода в Москве?".into()),
        &[],
        "",
        "ru",
        false,
    );
    assert!(sys.contains("не про техническую"));
}

/// Keyword trigger produces a user-prompt with the keyword + line.
#[test]
fn prompt_keyword_trigger_includes_keyword_and_line() {
    let (_sys, usr) = build_auto_tile_prompts(
        &Trigger::Keyword("etcd".into(), "мы используем etcd для consensus".into()),
        &[],
        "",
        "ru",
        false,
    );
    assert!(usr.contains("etcd"));
    assert!(usr.contains("consensus"));
}

/// Live-coaching mode adds read-aloud tone rules to the system prompt; off leaves
/// them out (Фича1). The two modes must be independent of everything else.
#[test]
fn prompt_live_coaching_adds_readaloud_rules() {
    let q = Trigger::Question("как ответить?".into());
    let (on, _) = build_auto_tile_prompts(&q, &[], "", "ru", true);
    let (off, _) = build_auto_tile_prompts(&q, &[], "", "ru", false);
    assert!(
        on.contains("чтения вслух"),
        "live=on must add read-aloud rules"
    );
    assert!(on.contains("без слов-паразитов"));
    assert!(
        !off.contains("чтения вслух"),
        "live=off must NOT add read-aloud rules"
    );
}

/// Reask with no prior QA → emits tile:error + returns None.
/// Verifies the short-circuit path doesn't try to call AI.
#[tokio::test]
async fn reask_last_no_prior_qa_emits_error_and_returns_none() {
    let cfg = hermetic_empty_config();
    let inputs = ReaskInputs {
        last_question: None,
        last_answer: None,
        recent_transcript_iconized: vec![],
        journal: None,
        health: Arc::new(HealthSignals::default()),
    };
    let sink: Arc<dyn RuntimeEvents> = Arc::new(Noop);
    let outcome = reask_last(sink, cfg, inputs).await;
    assert!(outcome.is_none(), "no-prior-QA path must return None");
}

/// `ask_stream_loop` end-to-end with a hand-fed receiver:
/// 3 Deltas + 1 Done → accumulator hits 3 tokens, cost_apply
/// called exactly once with non-zero micro, Done emitted.
#[tokio::test]
async fn ask_stream_loop_processes_deltas_then_done_and_calls_cost_apply_once() {
    use std::sync::Mutex as StdMutex;
    let (tx, rx) = tokio::sync::mpsc::channel::<ai::AiEvent>(8);
    // Feed events from a separate task so the receiver loop drives.
    let feeder = tokio::spawn(async move {
        tx.send(ai::AiEvent::Delta {
            text: "Hello".into(),
        })
        .await
        .unwrap();
        tx.send(ai::AiEvent::Delta { text: " ".into() })
            .await
            .unwrap();
        tx.send(ai::AiEvent::Delta {
            text: "world".into(),
        })
        .await
        .unwrap();
        tx.send(ai::AiEvent::Done {
            reason: "stop".into(),
        })
        .await
        .unwrap();
        // Closing tx after Done is the natural shutdown.
    });

    let calls = Arc::new(StdMutex::new(Vec::<u64>::new()));
    let calls_clone = calls.clone();
    let cost_apply: CostApplyFn = Box::new(move |micro| {
        calls_clone.lock().unwrap().push(micro);
        0.0001234 // arbitrary USD total for the cost:update emit
    });

    let sink: Arc<dyn RuntimeEvents> = Arc::new(Noop);
    ask_stream_loop(
        sink,
        rx,
        "claude-haiku-4-5".into(),
        false, // cloud — bill normally
        "sys".into(),
        "usr".into(),
        None,
        Arc::new(HealthSignals::default()),
        std::time::Instant::now(),
        cost_apply,
    )
    .await;
    feeder.await.unwrap();

    let calls = calls.lock().unwrap();
    assert_eq!(
        calls.len(),
        1,
        "cost_apply must be called exactly once at end-of-stream"
    );
    assert!(
        calls[0] > 0,
        "estimated cost should be non-zero for 11-char accumulated answer"
    );
}

/// `ask_stream_loop` with immediate Error → cost_apply still
/// fires (output_tokens=0 → micro≈0) so the cost:update emit
/// remains parity-correct on the error path too. FIX #9: the Error
/// arm must also bump `health.last_ai_err_ms` (was 0) so HealthSignals
/// flips the bar to "AI down" — mirrors the non-streaming auto-tile path.
#[tokio::test]
async fn ask_stream_loop_error_path_calls_cost_apply_once_and_marks_ai_down() {
    use std::sync::Mutex as StdMutex;
    let (tx, rx) = tokio::sync::mpsc::channel::<ai::AiEvent>(2);
    let feeder = tokio::spawn(async move {
        tx.send(ai::AiEvent::Error {
            message: "stream died: upstream 503".into(),
        })
        .await
        .unwrap();
    });
    let calls = Arc::new(StdMutex::new(0u32));
    let calls_clone = calls.clone();
    let cost_apply: CostApplyFn = Box::new(move |_micro| {
        *calls_clone.lock().unwrap() += 1;
        0.0
    });
    let sink: Arc<dyn RuntimeEvents> = Arc::new(Noop);
    let health = Arc::new(HealthSignals::default());
    assert_eq!(
        health.last_ai_err_ms.load(Ordering::Relaxed),
        0,
        "precondition: no AI error recorded yet"
    );
    ask_stream_loop(
        sink,
        rx,
        "claude-haiku-4-5".into(),
        false,
        "sys".into(),
        "usr".into(),
        None,
        health.clone(),
        std::time::Instant::now(),
        cost_apply,
    )
    .await;
    feeder.await.unwrap();
    assert_eq!(*calls.lock().unwrap(), 1);
    assert!(
        health.last_ai_err_ms.load(Ordering::Relaxed) > 0,
        "FIX #9: streaming Error arm must bump last_ai_err_ms so the bar flips to AI down"
    );
}

/// FIX #4 — a LOCAL streamed answer must journal a ZERO cost, not the
/// Sonnet-fallback price. `ask_stream_loop` hands the SAME `micro` to both
/// `cost_apply` AND `JournalEvent::AiResponse`, so capturing the cost_apply
/// arg proves the journaled value too. An unknown local model id maps to
/// Sonnet pricing in `cost_microcents`, so WITHOUT the `is_local` gate this
/// non-empty answer would carry a phantom > 0 cost. With `is_local=true` it
/// must be exactly 0. (The non-`is_local` arm is covered by the cloud test
/// above, which asserts a non-zero estimate for the same shape of input.)
#[tokio::test]
async fn ask_stream_loop_local_journals_zero_cost() {
    use std::sync::Mutex as StdMutex;

    // Sanity: the model id we use really does fall back to a non-zero
    // (Sonnet) price, so a zero result can only come from the is_local gate.
    let phantom = ai::cost_microcents("my-local-gemma-3-it", 1000, 1000);
    assert!(
        phantom > 0,
        "precondition: an unknown local model id must fall back to a non-zero price"
    );

    let (tx, rx) = tokio::sync::mpsc::channel::<ai::AiEvent>(8);
    let feeder = tokio::spawn(async move {
        tx.send(ai::AiEvent::Delta {
            text: "a fairly long local answer with many tokens".into(),
        })
        .await
        .unwrap();
        tx.send(ai::AiEvent::Done {
            reason: "stop".into(),
        })
        .await
        .unwrap();
    });

    let billed = Arc::new(StdMutex::new(Vec::<u64>::new()));
    let billed_clone = billed.clone();
    let cost_apply: CostApplyFn = Box::new(move |micro| {
        billed_clone.lock().unwrap().push(micro);
        0.0
    });

    let sink: Arc<dyn RuntimeEvents> = Arc::new(Noop);
    ask_stream_loop(
        sink,
        rx,
        "my-local-gemma-3-it".into(),
        true, // local — must NOT bill / journal a cost
        "sys prompt".into(),
        "usr prompt".into(),
        None,
        Arc::new(HealthSignals::default()),
        std::time::Instant::now(),
        cost_apply,
    )
    .await;
    feeder.await.unwrap();

    let billed = billed.lock().unwrap();
    assert_eq!(billed.len(), 1, "cost_apply called exactly once");
    assert_eq!(
        billed[0], 0,
        "FIX #4: local answer must journal/bill 0, not the Sonnet-fallback price ({phantom} µ¢)"
    );
}

/// Manual spawn with empty transcript → spawns a feedback tile +
/// returns None. No AI call attempted.
#[tokio::test]
async fn manual_spawn_tile_empty_transcript_returns_none() {
    let cfg = hermetic_empty_config();
    let inputs = ManualSpawnInputs {
        recent_transcript_labeled: vec![],
        last_line: None,
        cost_cap_reason: None,
        journal: None,
        health: Arc::new(HealthSignals::default()),
    };
    let sink: Arc<dyn RuntimeEvents> = Arc::new(Noop);
    let outcome = manual_spawn_tile(sink, cfg, inputs).await;
    assert!(
        outcome.is_none(),
        "empty-transcript path must return None (got {outcome:?})"
    );
}

/// Manual spawn with a transcript line + over-budget cap +
/// hermetic AI config → cost:cap-hit fires (non-blocking), AI
/// call fails (empty bearer), outcome is None, no panic.
#[tokio::test]
async fn manual_spawn_tile_over_budget_warns_but_proceeds() {
    let cfg = hermetic_empty_config();
    let inputs = ManualSpawnInputs {
        recent_transcript_labeled: vec!["[ПОЛЬЗОВАТЕЛЬ] hello".into()],
        last_line: Some(TranscriptLine {
            source: AudioSource::Mic,
            text: "hello".into(),
            timestamp_ms: 0,
        }),
        cost_cap_reason: Some("over budget: $0.50 spent ≥ $0.10".into()),
        journal: None,
        health: Arc::new(HealthSignals::default()),
    };
    let sink: Arc<dyn RuntimeEvents> = Arc::new(Noop);
    let outcome = manual_spawn_tile(sink, cfg, inputs).await;
    // Cap-hit is non-blocking: port still attempts the AI call
    // (fails under hermetic config) → outcome None, no panic.
    assert!(outcome.is_none());
}

/// Reask with prior QA but explicitly-empty AI config → AI call
/// fails (no base_url / no bearer) → emits tile:error + returns
/// None. No panic, no real network hit.
#[tokio::test]
async fn reask_last_ai_error_returns_none_without_panic() {
    let cfg = hermetic_empty_config();
    let inputs = ReaskInputs {
        last_question: Some("How to scale Kubernetes?".into()),
        last_answer: Some("Use horizontal pod autoscaler.".into()),
        recent_transcript_iconized: vec!["mic: we need more pods".into()],
        journal: None,
        health: Arc::new(HealthSignals::default()),
    };
    let sink: Arc<dyn RuntimeEvents> = Arc::new(Noop);
    let outcome = reask_last(sink, cfg, inputs).await;
    // Hermetic config has empty ai_bearer / ai_base_url → AI fails
    // → outcome is None (error path). Reaching here means no panic.
    // Print the full Option<ReaskOutcome> on assertion failure (not
    // just `.is_some()` which would always be `true` here and tell
    // us nothing useful).
    assert!(
        outcome.is_none(),
        "AI-error path must return None (got {outcome:?})"
    );
}
