//! Runtime fns ported from `src-tauri/src/runtime.rs` (Phase B2).
//!
//! Each ported fn takes `Arc<dyn RuntimeEvents>` instead of
//! Tauri's `AppHandle` + `SharedTiles`. The src-tauri side keeps
//! thin shim wrappers that construct a `TauriEvents` adapter +
//! delegate here, so React/Tauri callers (Tauri command registry)
//! see zero signature changes during the migration.
//!
//! Port order per `docs/PHASE-B2-RUNTIME-PORT-PLAN.md`:
//!   #1 run_post_meeting_debrief   ← landed
//!   #2 reask_last                 (pending)
//!   #3 manual_spawn_tile          (pending)
//!   #4 ask                        (pending)
//!   #5 manual_ask_source          (pending)
//!   #6 manual_ask_window_end      (pending)
//!   #7 maybe_spawn_tile + start_session (together)
//!   #8 stop_session               (depends on debrief)

use crate::ai;
use crate::audio::{AudioSource, TranscriptLine};
use crate::config::SharedConfig;
use crate::events::{MonitorHint, RuntimeEvents, TileKind, TileSpec};
use std::sync::Arc;

/// Post-meeting debrief — run Sonnet over the user's MIC-side
/// transcript for 3 concise coaching observations: pace, fillers,
/// dominance, story structure, vocabulary. Spawn the result as a
/// debrief tile so the user sees it after the meeting ends.
/// Fire-and-forget — if the AI call fails for any reason, log + drop.
///
/// Port #1 of Phase B2 (smallest, private, 0 emits, 1 tile spawn).
/// Replaces `tile::spawn_tile_with_stealth` with the trait method
/// `events.spawn_tile_full(...)` which carries `TileKind::Debrief` +
/// the session-wide stealth flag.
pub async fn run_post_meeting_debrief(
    events: Arc<dyn RuntimeEvents>,
    cfg: SharedConfig,
    transcript: Vec<TranscriptLine>,
) {
    let (base_url, bearer, model, response_language, preferred_monitor, stealth) = {
        let c = cfg.read();
        (
            c.ai_base_url.clone(),
            c.ai_bearer.clone(),
            c.prep_model.clone(),
            c.response_language.clone(),
            c.tile_monitor_name.clone(),
            c.stealth_enabled,
        )
    };
    // Mic-only transcript for "you" coaching. Snapshot is already capped
    // at TRANSCRIPT_MAX_LINES (=80) upstream so no second cap needed.
    let mic_text: String = transcript
        .iter()
        .filter(|l| matches!(l.source, AudioSource::Mic))
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    // Localise BOTH the prompt body and the tile title — Russian-only
    // output would be confusing for an English-speaking user even with
    // a trailing "Respond in English" directive.
    let is_ru = response_language == "ru";
    let system_prompt = if is_ru {
        "Ты — speech coach. На входе — полный mic-транскрипт пользователя за встречу \
         (только реплики самого пользователя, без собеседника). \
         Дай РОВНО 3 конкретных наблюдения о его манере речи: \
         (1) ритм/темп, (2) слова-паразиты, (3) структура высказываний / уверенность. \
         Каждое наблюдение в формате: одно короткое предложение + 1-2 примера ИЗ ТРАНСКРИПТА в кавычках. \
         Если транскрипт слишком короткий/пустой для какого-то аспекта — честно скажи 'недостаточно данных'. \
         Не хвали зря, не пиши воды. Отвечай на русском языке."
            .to_string()
    } else {
        "You are a speech coach. The input is the user's full mic transcript from a meeting \
         (their own lines only, no interlocutor). \
         Provide EXACTLY 3 specific observations about their speaking: \
         (1) pace/rhythm, (2) filler words, (3) structure / confidence. \
         Each observation: one short sentence + 1-2 verbatim QUOTES FROM THE TRANSCRIPT. \
         If the transcript is too short/empty for any aspect, say so honestly. \
         No empty praise, no filler. Respond in English."
            .to_string()
    };
    let messages = vec![
        ai::ChatMessage {
            role: "system".into(),
            content: ai::MessageContent::Text(system_prompt),
        },
        ai::ChatMessage {
            role: "user".into(),
            content: ai::MessageContent::Text(mic_text),
        },
    ];
    let answer = match ai::complete(&base_url, &bearer, &model, messages, 1024).await {
        Ok(text) => text,
        Err(e) => {
            log::warn!("post-meeting debrief AI call failed: {e:#}");
            return;
        }
    };
    log::info!("post-meeting debrief landed: {} chars", answer.len());

    let tile_title = if is_ru {
        "🎯 Debrief: что улучшить".to_string()
    } else {
        "🎯 Debrief: what to improve".to_string()
    };
    // Carry the OS-side monitor-name pin from cfg through the trait
    // boundary. The Tauri adapter passes the name straight into
    // `tile::pick_monitor(name)` for exact match; the Slint adapter
    // ignores Named today (no enumerator yet) and falls back to Auto.
    // Empty/None → Auto so we don't burden either side with empty-string
    // edge cases.
    let monitor_hint = match preferred_monitor.as_deref() {
        Some(name) if !name.is_empty() => MonitorHint::Named(name.to_string()),
        _ => MonitorHint::Auto,
    };

    if let Err(e) = events.spawn_tile_full(
        TileSpec {
            question: tile_title,
            answer,
            source: "debrief".into(),
            is_translation: false,
        },
        monitor_hint,
        stealth,
        TileKind::Debrief,
    ) {
        log::warn!("post-meeting debrief tile spawn failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Noop;

    /// Smoke test that the debrief ports compiles + runs with Noop
    /// events sink. With a bogus AI bridge config the call short-
    /// circuits on the AI error path (no tile spawned), but we
    /// verify the fn doesn't panic + returns.
    #[tokio::test]
    async fn run_post_meeting_debrief_with_noop_events_does_not_panic() {
        let cfg = crate::config::shared();
        let transcript = vec![TranscriptLine {
            source: AudioSource::Mic,
            text: "test utterance".into(),
            timestamp_ms: 0,
        }];
        let sink: Arc<dyn RuntimeEvents> = Arc::new(Noop);
        // Fire-and-forget — if config has no AI bridge set, the AI
        // call fails and the fn returns without spawning a tile.
        // Either way no panic, no resource leak.
        run_post_meeting_debrief(sink, cfg, transcript).await;
    }
}
