//! Deep lock for the app-managed local AI server (bar lock chip, v0.37).
//!
//! The bar's lock chip is a THREE-state control, but ONLY when the active AI
//! endpoint is Suflyor's own managed llama-server (loopback :8080 — the exact
//! [`crate::local_ai::is_managed_llama_endpoint`] ownership rules). Cloud and
//! external/Ollama endpoints keep the legacy two-state toggle (listening mode
//! on/off) and their processes are never stopped:
//!
//! 1. unlocked → click → LISTENING: existing `suppress_tiles` on, nothing else;
//! 2. listening → click → DEEP LOCK: `suppress_tiles` stays on, the app-owned
//!    llama-server on :8080 is unloaded owner-aware to free RAM/VRAM;
//! 3. deep-locked → click → UNLOCK: start the selected managed model and wait
//!    for full readiness; ONLY on success clear deep lock + `suppress_tiles`.
//!    On failure keep both and show a concise localized error.
//!
//! `Config::deep_lock` persists state 3 across restart; the process-wide
//! [`deep_lock_active`] flag mirrors it at runtime so the low-level request
//! guard in [`crate::ai`] can refuse managed-local traffic from ANY path
//! (tiles, hotkeys, PTT, summary, naming, Settings pings, …) without touching
//! the config lock. Recording, local STT (:8081), TTS and OCR are unaffected.

use std::sync::atomic::{AtomicBool, Ordering};

/// Runtime mirror of `Config::deep_lock`. Set at boot from the persisted
/// config and on every lock-chip transition; the request guard reads it.
static DEEP_LOCK_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_deep_lock_active(active: bool) {
    DEEP_LOCK_ACTIVE.store(active, Ordering::Release);
}

#[must_use]
pub fn deep_lock_active() -> bool {
    DEEP_LOCK_ACTIVE.load(Ordering::Acquire)
}

/// Error marker carried by every request the deep lock refused. Stable text —
/// high-level feedback sites match on it via [`is_blocked_error`] to swap in a
/// localized notice. Privacy-safe by construction (no URL/host).
pub const BLOCKED_ERROR: &str = "local AI is deep-locked";

#[must_use]
pub fn is_blocked_error(msg: &str) -> bool {
    msg.contains(BLOCKED_ERROR)
}

/// Pure guard predicate used by [`crate::ai`]: block only when the deep lock
/// is active AND the target is the app-managed loopback llama endpoint. Cloud,
/// external Ollama, whisper :8081 and any other URL pass through untouched.
#[must_use]
pub fn endpoint_blocked(deep_lock: bool, base_url: &str) -> bool {
    deep_lock && crate::local_ai::is_managed_llama_endpoint(base_url)
}

/// Ordinary managed-server lifecycle paths are blocked while deep-locked.
/// Only the explicit third LockChip click may opt into the unlock launch.
#[must_use]
pub const fn lifecycle_launch_allowed(deep_lock: bool, explicit_unlock: bool) -> bool {
    !deep_lock || explicit_unlock
}

/// True when the config's ACTIVE live endpoint is the app-managed local
/// server — the only case where the chip runs the three-state machine.
#[must_use]
pub fn cfg_is_managed_local(cfg: &crate::config::Config) -> bool {
    cfg.ai_provider == "local" && crate::local_ai::is_managed_llama_endpoint(&cfg.ai_local_base_url)
}

/// What a lock-chip click does next. Pure so the transition table is testable
/// without UI/process state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockAction {
    /// Cloud / external (e.g. Ollama) endpoint: the legacy two-state toggle of
    /// `suppress_tiles` only. External processes are never stopped.
    ToggleSuppress,
    /// Managed local, unlocked → enable listening mode (`suppress_tiles` on).
    EnableListening,
    /// Managed local, listening → enter deep lock: keep `suppress_tiles` on
    /// and unload the app-owned llama-server on :8080.
    EnterDeepLock,
    /// Managed local, deep-locked → start the selected managed model back up;
    /// clear deep lock + `suppress_tiles` only on confirmed readiness.
    Unlock,
}

#[must_use]
pub fn next_lock_action(
    is_managed_local: bool,
    suppress_tiles: bool,
    deep_lock: bool,
) -> LockAction {
    if !is_managed_local {
        return LockAction::ToggleSuppress;
    }
    if deep_lock {
        LockAction::Unlock
    } else if suppress_tiles {
        LockAction::EnterDeepLock
    } else {
        LockAction::EnableListening
    }
}

/// Localized lock-chip description (accessible label / tooltip substitute) for
/// each state — the chip itself has no hover tooltip surface, so the label is
/// the discoverable per-state copy. No hidden gestures.
#[must_use]
pub fn state_hint(ru: bool, managed: bool, suppress_tiles: bool, deep_lock: bool) -> &'static str {
    if !managed {
        return match (ru, suppress_tiles) {
            (true, false) => "Блокировка тайлов — режим прослушивания (без всплывающих окон)",
            (true, true) => "Режим прослушивания включён — нажмите, чтобы снять блокировку",
            (false, false) => "Lock tiles — stop pop-ups (listening mode)",
            (false, true) => "Listening mode on — press to unlock",
        };
    }
    match (ru, deep_lock, suppress_tiles) {
        (_, true, _) => {
            if ru {
                "Глубокая блокировка: локальный ИИ выгружен. Нажмите, чтобы запустить модель и снять блокировку"
            } else {
                "Deep lock: local AI is unloaded. Press to start the model and unlock"
            }
        }
        (true, false, true) => {
            "Режим прослушивания включён. Нажмите ещё раз для глубокой блокировки (локальный ИИ будет выгружен)"
        }
        (false, false, true) => {
            "Listening mode on. Press again to deep-lock (the local AI will be unloaded)"
        }
        (true, false, false) => {
            "Блокировка тайлов: первое нажатие — режим прослушивания, второе — глубокая блокировка с выгрузкой локального ИИ"
        }
        (false, false, false) => {
            "Lock tiles: first press enables listening mode, second press deep-locks and unloads the local AI"
        }
    }
}

/// Short bar status-line copy for each transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockStatus {
    ListeningOn,
    ListeningOff,
    Unloading,
    Unloaded,
    Unlocking,
    UnlockReady,
    UnlockFailed,
}

#[must_use]
pub fn status_text(ru: bool, status: LockStatus) -> &'static str {
    match (ru, status) {
        (true, LockStatus::ListeningOn) => "прослушивание",
        (false, LockStatus::ListeningOn) => "listening",
        (true, LockStatus::ListeningOff) => "разблокировано",
        (false, LockStatus::ListeningOff) => "unlocked",
        (true, LockStatus::Unloading) => "выгрузка ИИ…",
        (false, LockStatus::Unloading) => "unloading AI…",
        (true, LockStatus::Unloaded) => "ИИ выгружен",
        (false, LockStatus::Unloaded) => "AI unloaded",
        (true, LockStatus::Unlocking) => "запуск модели…",
        (false, LockStatus::Unlocking) => "starting model…",
        (true, LockStatus::UnlockReady) => "ИИ готов",
        (false, LockStatus::UnlockReady) => "AI ready",
        (true, LockStatus::UnlockFailed) => "не удалось запустить ИИ",
        (false, LockStatus::UnlockFailed) => "AI start failed",
    }
}

/// Localized notice shown wherever a managed-local request was refused by the
/// deep lock (tiles, Settings statuses, context structuring, …).
#[must_use]
pub fn blocked_notice(ru: bool) -> &'static str {
    if ru {
        "Локальный ИИ заблокирован (значок замка на панели). Снимите блокировку, чтобы получать ответы."
    } else {
        "Local AI is deep-locked (the bar's lock chip). Unlock it to get answers."
    }
}

/// User-facing test/diagnostics rendering for the stable blocked marker.
/// `None` lets callers keep their existing generic error handling.
#[must_use]
pub fn blocked_test_result(ru: bool, msg: &str) -> Option<String> {
    is_blocked_error(msg).then(|| format!("[--] {}", blocked_notice(ru)))
}

/// Localized notice for a FAILED unlock attempt (the lock + suppression stay).
#[must_use]
pub fn unlock_failed_notice(ru: bool) -> &'static str {
    if ru {
        "Не удалось запустить локальный ИИ — блокировка сохранена. Проверьте установку в Настройки → AI и попробуйте ещё раз."
    } else {
        "Couldn't start the local AI — the lock is kept. Check Settings → AI and try again."
    }
}

/// Localized Settings status when a server lifecycle op (install / model
/// switch / context restart / engine update) is refused because the deep lock
/// is active.
#[must_use]
pub fn lifecycle_guard_notice(ru: bool) -> &'static str {
    if ru {
        "Глубокая блокировка активна — сначала снимите её значком замка на панели."
    } else {
        "Deep lock is active — unlock from the bar's lock chip first."
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::config::Config;

    fn cfg(provider: &str, base_url: &str) -> Config {
        let mut c = Config::defaults();
        c.ai_provider = provider.to_string();
        c.ai_local_base_url = base_url.to_string();
        c
    }

    #[test]
    fn managed_only_for_local_provider_on_bundled_loopback_endpoint() {
        assert!(cfg_is_managed_local(&cfg(
            "local",
            "http://127.0.0.1:8080/v1"
        )));
        // External Ollama on the same provider stays two-state.
        assert!(!cfg_is_managed_local(&cfg(
            "local",
            "http://127.0.0.1:11434/v1"
        )));
        // A LAN host on :8080 is not ours.
        assert!(!cfg_is_managed_local(&cfg(
            "local",
            "http://192.168.0.10:8080/v1"
        )));
        // Cloud provider never runs the three-state machine.
        assert!(!cfg_is_managed_local(&cfg(
            "cloud",
            "http://127.0.0.1:8080/v1"
        )));
    }

    #[test]
    fn managed_clicks_walk_three_states() {
        // unlocked -> listening -> deep lock -> unlock -> (repeat).
        assert_eq!(
            next_lock_action(true, false, false),
            LockAction::EnableListening
        );
        assert_eq!(
            next_lock_action(true, true, false),
            LockAction::EnterDeepLock
        );
        assert_eq!(next_lock_action(true, true, true), LockAction::Unlock);
    }

    #[test]
    fn non_managed_clicks_keep_the_two_state_toggle() {
        assert_eq!(
            next_lock_action(false, false, false),
            LockAction::ToggleSuppress
        );
        assert_eq!(
            next_lock_action(false, true, false),
            LockAction::ToggleSuppress
        );
        // Even if a stale deep_lock flag is present, a cloud/external chip
        // never runs the managed machine.
        assert_eq!(
            next_lock_action(false, true, true),
            LockAction::ToggleSuppress
        );
    }

    #[test]
    fn endpoint_guard_blocks_only_managed_url_while_active() {
        assert!(endpoint_blocked(true, "http://127.0.0.1:8080/v1"));
        assert!(endpoint_blocked(true, "http://localhost:8080/v1"));
        // Unlocked: everything passes.
        assert!(!endpoint_blocked(false, "http://127.0.0.1:8080/v1"));
        // Locked but not the managed endpoint: cloud / Ollama / whisper pass.
        assert!(!endpoint_blocked(true, "http://192.168.0.142:18902/v1"));
        assert!(!endpoint_blocked(true, "http://127.0.0.1:11434/v1"));
        assert!(!endpoint_blocked(true, "http://127.0.0.1:8081/v1"));
    }

    #[test]
    fn lifecycle_guard_has_one_explicit_unlock_bypass() {
        assert!(lifecycle_launch_allowed(false, false));
        assert!(lifecycle_launch_allowed(false, true));
        assert!(!lifecycle_launch_allowed(true, false));
        assert!(lifecycle_launch_allowed(true, true));
    }

    #[test]
    fn blocked_error_marker_matches_its_chains() {
        // Pure string contract — the global flag itself is exercised by the
        // ai.rs guard test (kept serial there); touching it here would race.
        assert!(is_blocked_error(BLOCKED_ERROR));
        assert!(is_blocked_error(&format!("context: {BLOCKED_ERROR}")));
        assert!(!is_blocked_error("AI connection error"));
        assert!(!is_blocked_error(""));
    }

    #[test]
    fn copy_is_localized_and_state_distinct() {
        // Every state has RU AND EN copy, and the states don't share it.
        let hint_unlocked = state_hint(true, true, false, false);
        let hint_listening = state_hint(true, true, true, false);
        let hint_deep = state_hint(true, true, true, true);
        assert_ne!(hint_unlocked, hint_listening);
        assert_ne!(hint_listening, hint_deep);
        assert!(!hint_unlocked.is_empty());
        for ru in [true, false] {
            assert!(!state_hint(ru, false, false, false).is_empty());
            assert!(!blocked_notice(ru).is_empty());
            assert!(!unlock_failed_notice(ru).is_empty());
            assert!(!lifecycle_guard_notice(ru).is_empty());
            for status in [
                LockStatus::ListeningOn,
                LockStatus::ListeningOff,
                LockStatus::Unloading,
                LockStatus::Unloaded,
                LockStatus::Unlocking,
                LockStatus::UnlockReady,
                LockStatus::UnlockFailed,
            ] {
                assert!(!status_text(ru, status).is_empty());
            }
        }
        assert_ne!(blocked_notice(true), blocked_notice(false));
        assert_ne!(
            status_text(true, LockStatus::Unloaded),
            status_text(false, LockStatus::Unloaded)
        );
    }
}
