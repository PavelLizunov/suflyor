//! Source guard for the macOS text-ask + session-tile vertical slice.
//!
//! Pins the honesty contract of the macOS product slice without building any
//! UI: the bootstrap row only exposes wired controls, the setup window masks
//! its token and never re-populates it, failures stay generic, exactly one
//! reusable TextAskWindow/TileWindow exists (shared by manual asks and
//! session auto tiles through ONE bounded queue drained on the Slint main
//! thread), the slice stays on portable backend APIs, and the Windows
//! orchestration stays excluded.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn bootstrap_row_only_wires_the_connected_controls() {
    let bar = read(root(), "ui/overlay_bar.slint");
    let bootstrap = bar
        .split_once("if root.bootstrap-mode : VerticalLayout")
        .expect("bootstrap row exists")
        .1;

    // The three connected product chips + the existing Quit stay wired.
    assert!(bootstrap.contains("clicked => { root.mic-toggle-clicked(); }"));
    assert!(bootstrap.contains("clicked => { root.text-ask-clicked(); }"));
    assert!(bootstrap.contains("clicked => { root.open-settings-clicked(); }"));
    assert!(bootstrap.contains("clicked => { root.quit-confirm(); }"));
    assert!(bootstrap.contains("root.drag-start-requested();"));

    // One compact transcript/status row: the shared latest-line properties
    // with a translated placeholder — no new callbacks, no new layout modes.
    assert!(bootstrap.contains("root.last-transcript-line"));
    assert!(bootstrap.contains("root.last-transcript-source"));
    assert!(bootstrap.contains("@tr(\"waiting for transcript... (mic/sys → STT)\")"));
    let po = read(root(), "translations/ru/LC_MESSAGES/slint-replay.po");
    assert!(po.contains("msgid \"waiting for transcript... (mic/sys → STT)\""));

    // Nothing else from the product bar may surface in bootstrap mode.
    for forbidden in [
        "sys-toggle-clicked",
        "spawn-tile-clicked",
        "capture-clicked",
        "stealth-toggle-clicked",
        "aggressive-toggle-clicked",
        "lock-menu-opened",
        "lock-mode-selected",
        "archive-clicked",
        "summary-clicked",
        "help-clicked",
        "pause-toggle-clicked",
        "timer-toggle-clicked",
        "hide-to-tray-clicked",
        "restart-clicked",
        "close-all-tiles-clicked",
        "compact-toggle-clicked",
        "ptt-mic-pressed",
        "ptt-sys-pressed",
    ] {
        assert!(
            !bootstrap.contains(forbidden),
            "bootstrap row exposes an unconnected control: {forbidden}"
        );
    }
}

#[test]
fn setup_window_masks_the_token_and_keeps_words_in_tr() {
    let setup = read(root(), "ui/macos_ai_setup.slint");
    assert!(setup.contains("input-type: password;"));
    assert!(setup.contains("callback save-clicked();"));
    assert!(setup.contains("callback cancel-clicked();"));
    // The host drives the status line with a code; the words stay in @tr.
    assert!(setup.contains("in property <int> status-kind: 0;"));
    assert!(setup.contains("@tr(\"The AI bridge is not configured yet.\")"));
    // The typed token is the ONLY thing bound to token-input; nothing in the
    // UI seeds it from anywhere else.
    assert_eq!(setup.matches("root.token-input").count(), 1);
}

#[test]
fn setup_local_stt_field_is_optional_translated_and_not_a_token() {
    let setup = read(root(), "ui/macos_ai_setup.slint");
    // Exactly ONE optional plain-text field bound to local-stt-url.
    assert_eq!(setup.matches("root.local-stt-url").count(), 1);
    assert!(setup.contains("in-out property <string> local-stt-url;"));
    // Its label is translated; the field is NOT password-masked and there is
    // still exactly one password field in the whole window (the bridge token).
    assert!(setup.contains("@tr(\"Local STT service URL (optional)\")"));
    assert_eq!(setup.matches("input-type: password;").count(), 1);

    let po = read(root(), "translations/ru/LC_MESSAGES/slint-replay.po");
    assert!(po.contains("msgid \"Local STT service URL (optional)\""));
}

#[test]
fn local_stt_url_is_never_prefilled_and_only_saved_explicitly() {
    let module = read(root(), "src/bin/overlay_host/macos_text_ask.rs");

    // The stored URL is shown ONLY while "uap" is the active provider — the
    // ordinary whisper default can never prefill the optional field.
    assert!(module.contains("fn local_stt_display_url("));
    assert!(module.contains("if cfg.stt_provider == \"uap\""));
    assert!(module.contains("set_local_stt_url("));

    // Save selects the EXPLICIT "uap" provider + reuses stt_whisper_url; it
    // never touches any unrelated secret/config field.
    assert!(module.contains("fn apply_local_stt_url("));
    assert!(module.contains("cfg.stt_provider = \"uap\".to_string();"));
    assert!(module.contains("cfg.stt_whisper_url = base;"));

    // Validation goes through the shared normalizer; empty input is accepted
    // and keeps the stored STT config untouched.
    assert!(module.contains("fn local_stt_ready("));
    assert!(module.contains("normalize_uap_base_url("));

    // The live session still owns the ONLY config reload before start — the
    // setup window must not introduce a second reload path into the session.
    assert!(!module.contains("refresh_config_from_disk"));
    let session = read(root(), "src/bin/overlay_host/macos_session.rs");
    assert!(session.contains("refresh_config_from_disk"));
}

#[test]
fn stored_token_never_populates_the_field_and_is_cleared_on_every_exit() {
    let module = read(root(), "src/bin/overlay_host/macos_text_ask.rs");

    // Every write to the token field must clear it, never populate it.
    let setters: Vec<&str> = module
        .lines()
        .filter(|line| line.contains("set_token_input("))
        .collect();
    // At least: every Save attempt, Cancel, and the reopen refresh.
    assert!(
        setters.len() >= 3,
        "token field must be cleared on Save, Cancel, and reopen"
    );
    for line in &setters {
        assert!(
            line.contains("SharedString::default()"),
            "token field may only ever be cleared, never populated: {line}"
        );
    }
    // The save path reads the TYPED token, not a stored one.
    assert!(module.contains("get_token_input("));
    assert!(module.contains("get_bridge_url("));
    // A cancelled typed question never survives either.
    assert!(module.contains("set_query("));
    // The bearer never reaches the log.
    for line in module.lines() {
        if line.contains("logging::line") {
            assert!(
                !line.contains("bearer") && !line.contains("token"),
                "credential material near a log line: {line}"
            );
        }
    }
}

#[test]
fn ask_failures_stay_fixed_generic_and_unlogged() {
    let module = read(root(), "src/bin/overlay_host/macos_text_ask.rs");
    assert!(module.contains("fn ask_failure_copy(is_ru: bool) -> &'static str"));
    assert!(module.contains("fn asking_copy(is_ru: bool) -> &'static str"));
    // The completion error is deliberately unreadable: no URL, host, token,
    // or raw backend text can leak into the tile or the log.
    assert!(module.contains("Err(_) =>"));
    // Category-only breadcrumbs.
    assert!(module.contains("\"[macos] text-ask bridge request failed\""));
    for line in module.lines() {
        assert!(
            !line.contains("bridge request failed:"),
            "raw request error leaked into a log line: {line}"
        );
    }
}

#[test]
fn slice_keeps_exactly_one_reusable_window_of_each_kind() {
    let module = read(root(), "src/bin/overlay_host/macos_text_ask.rs");
    assert_eq!(module.matches("TextAskWindow::new()").count(), 1);
    assert_eq!(module.matches("TileWindow::new()").count(), 1);
    assert_eq!(module.matches("MacAiSetupWindow::new()").count(), 1);
    assert!(module.contains("RefCell<Option<ui::TextAskWindow>>"));
    assert!(module.contains("RefCell<Option<ui::TileWindow>>"));
    assert!(module.contains("RefCell<Option<ui::MacAiSetupWindow>>"));
    // Windows are hidden and reused, never dropped per ask.
    assert!(module.contains("window.hide();"));
    assert!(!module.contains("drop(tile"));
}

#[test]
fn slice_stays_on_portable_backend_apis() {
    let module = read(root(), "src/bin/overlay_host/macos_text_ask.rs");
    assert!(module.contains("overlay_backend::config::load()"));
    assert!(module.contains("overlay_backend::config::save("));
    assert!(module.contains("slint_replay::markdown::parse("));
    assert!(module.contains("slint::invoke_from_event_loop("));

    // Live-answer API with an explicit legacy bridge endpoint — NOT the
    // structuring-mode `complete`, NOT the provider routing of ai_endpoint().
    assert!(module.contains("overlay_backend::ai::complete_with_usage_endpoint("));
    assert!(module.contains("AiProtocol::OpenAiCompatible"));
    assert!(!module.contains("ai::complete("));
    assert!(!module.contains("ai_endpoint("));
    // The Windows one-shot live-answer budget, not the streaming cap.
    assert!(module.contains("const ANSWER_MAX_TOKENS: u32 = 600;"));
    // Readiness covers URL + token + model.
    assert!(module.contains("!cfg.ai_model.trim().is_empty()"));

    // No Windows orchestration and no recreated streaming bridge. (The
    // portable `native::clipboard::set_text` copy path is allowed — its own
    // contract is pinned by `macos_clipboard_copy_keeps_its_contracts`.)
    for forbidden in [
        "stream_chat",
        "win32::",
        "HWND",
        "hotkey",
        "overlay_host_windows",
    ] {
        assert!(
            !module.contains(forbidden),
            "the macOS slice must stay portable, found: {forbidden}"
        );
    }
}

#[test]
fn windows_include_stays_isolated_from_the_macos_slice() {
    let host = read(root(), "src/bin/overlay_host.rs");
    // The Windows orchestration stays behind its cfg gate, untouched.
    assert!(host.contains("#[cfg(windows)]"));
    assert!(host.contains("include!(\"overlay_host_windows.rs\");"));
    let windows_gate = host
        .split_once("include!(\"overlay_host_windows.rs\");")
        .expect("windows include exists")
        .0;
    assert!(windows_gate.trim_end().ends_with("#[cfg(windows)]"));
    // The mac modules are gated to macOS only.
    assert!(host.contains("#[cfg(target_os = \"macos\")]"));
    assert!(host.contains("#[path = \"overlay_host/macos_text_ask.rs\"]"));
    assert!(host.contains("mod macos_text_ask;"));
    let mac_gate = "\
#[cfg(target_os = \"macos\")]
#[path = \"overlay_host/macos_text_ask.rs\"]
mod macos_text_ask;";
    assert!(host.contains(mac_gate));
    let session_gate = "\
#[cfg(target_os = \"macos\")]
#[path = \"overlay_host/macos_session.rs\"]
mod macos_session;";
    assert!(host.contains(session_gate));
    // The bootstrap surface stays on.
    assert!(host.contains("set_bootstrap_mode(false)"));
    assert!(host.contains("set_mic_permission_state("));
    assert!(host.contains("set_mic_capture_state("));
    assert!(host.contains("on_mic_toggle_clicked("));
    assert!(host.contains("request_microphone_permission("));
    // The mic chip drives the shared transcript session — the startup
    // config is wrapped into the shared handle, start/stop run through the
    // session module, and the quit path stops + drains it deterministically.
    assert!(host.contains("overlay_backend::config::shared_from(cfg.clone())"));
    assert!(host.contains("macos_session::MacTranscriptSession::new("));
    // Session tiles cross the pipeline→UI thread boundary through ONE bounded
    // queue created in main; a Repeated Slint Timer drains it nonblocking
    // (at most one event per tick) into the shared auto-tile presenter.
    assert!(host.contains("std::sync::mpsc::sync_channel::<macos_session::MacTileSpawn>(8)"));
    assert!(host.contains("macos_session::MacTileSpawner::new(tile_tx)"));
    assert!(host.contains("slint::TimerMode::Repeated"));
    assert!(host.contains("tile_rx.try_recv()"));
    assert!(host.contains(".present_auto_tile("));
    assert!(host.contains("session.is_active()"));
    assert!(host.contains("session.start()"));
    assert!(host.contains("session.stop()"));
    assert!(host.contains("session.shutdown()"));
    // The raw drain spike is gone for good.
    assert!(!host.contains("blocking_recv"));
    assert!(!host.contains("stop_macos_mic_session"));
    assert!(!host.contains("macos-mic-drain"));
    assert!(!host.contains("overlay_backend::audio::start_capture("));
    // AppKit status Hide/Show/Quit + singleton stay owned by the macOS main.
    assert!(host.contains("slint_replay::native::status::install"));
    assert!(host.contains("slint_replay::native::lifecycle::acquire_singleton"));
}

#[test]
fn macos_session_reuses_the_shared_orchestrator_directly() {
    let module = read(root(), "src/bin/overlay_host/macos_session.rs");

    // Direct reuse of the shared session orchestrator — the same start/stop
    // the Windows host runs — on one owned Tokio runtime with the shared
    // runtime state, config handle and event trait.
    assert!(module.contains("slint_session::start_session("));
    assert!(module.contains("slint_session::stop_session("));
    assert!(module.contains("tokio::runtime::Builder::new_multi_thread()"));
    assert!(
        module.contains("slint_replay::runtime_state::{lock, shared_runtime, SharedSlintRuntime}")
    );
    assert!(module.contains("overlay_backend::config::SharedConfig"));
    assert!(module.contains("Arc<dyn RuntimeEvents>"));

    // Latest UI events cross through Weak + invoke_from_event_loop only.
    assert!(module.contains("slint::Weak<ui::OverlayBarWindow>"));
    assert!(module.contains("slint::invoke_from_event_loop("));
    assert!(module.contains("impl SlintUiBridge for MacSessionBridge"));
    // transcript:line lands in the existing last-transcript properties.
    assert!(module.contains("\"transcript:line\""));
    assert!(module.contains("set_last_transcript_line("));
    assert!(module.contains("set_last_transcript_source("));
    // Session lifecycle drives the honest chip state; every real stop emits.
    assert!(module.contains("\"session:started\""));
    assert!(module.contains("\"session:stopped\""));
    assert!(module.contains("set_mic_capture_state("));
    assert!(module.contains("self.events.emit(\"session:stopped\", serde_json::Value::Null);"));
    // Every start re-reads the on-disk config; a failed start cleans up.
    assert!(module.contains("overlay_backend::config::load()"));
    assert!(module.contains("*self.cfg.write() = fresh;"));

    // Tiles enqueue nonblocking into the bounded MacTileSpawner queue — the
    // old unsupported answer is gone, and no tile UI type lives here: the
    // single reusable TileWindow stays in macos_text_ask.
    assert!(module.contains("fn schedule_spawn_tile("));
    assert!(module.contains("self.tile_spawner.spawn(spec, kind)"));
    assert!(module.contains("struct MacTileSpawner"));
    assert!(module.contains("tx: SyncSender<MacTileSpawn>"));
    assert!(module.contains("self.tx.try_send("));
    assert!(!module.contains("tile windows are not supported on macOS yet"));
    assert!(!module.contains("TileWindow"));

    // No duplicated STT/audio/journal pipeline, no throttled-away lines,
    // no Windows APIs, no second async stop path.
    for forbidden in [
        "stt::spawn",
        "audio::start_capture",
        "blocking_recv",
        "Journal::",
        "win32::",
        "HWND",
        "hotkey",
        "overlay_host_windows",
        "stop_sync",
        "handle.spawn",
        "TRANSCRIPT_MIN_INTERVAL",
    ] {
        assert!(
            !module.contains(forbidden),
            "the macOS session must stay a thin reuse of slint_session, found: {forbidden}"
        );
    }

    // Secrets never reach a log line in this module.
    for line in module.lines() {
        if line.contains("logging::line") || line.contains("eprintln") {
            for leak in ["bearer", "token", "api_key", "groq_api_key"] {
                assert!(
                    !line.contains(leak),
                    "credential material near a log line: {line}"
                );
            }
        }
    }
}

#[test]
fn present_auto_tile_presents_final_answers_without_extra_ai_work() {
    let module = read(root(), "src/bin/overlay_host/macos_text_ask.rs");
    let body = module
        .split_once("pub(super) fn present_auto_tile(")
        .expect("present_auto_tile exists")
        .1
        .split_once("#[cfg(test)]")
        .expect("present_auto_tile ends before the inline test module")
        .0;

    // The carried answer is validated BEFORE anything gets invalidated, and
    // the SAME generation counter start_ask uses is bumped so a stale manual
    // completion still in flight cannot overwrite the auto tile.
    let validate = body
        .find("final_answer(&spec.answer)")
        .expect("auto tile validates its carried answer");
    let invalidate = body
        .find("self.generation.fetch_add(1, Ordering::Relaxed)")
        .expect("auto tile bumps the generation counter");
    assert!(
        validate < invalidate,
        "the answer must be validated before the generation is bumped"
    );
    assert!(module.contains("generation: Arc<AtomicU64>"));
    assert!(
        module.contains("let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;")
    );

    // The reused window's copy + trigger state is reset explicitly.
    assert!(body.contains("set_can_copy(true)"));
    assert!(body.contains("set_copied(false)"));
    assert!(body.contains("set_copied_block_index(-1)"));
    assert!(body.contains("set_trigger_label(SharedString::default())"));

    // The answer arrived final: no AI completion and no re-posting across
    // threads inside the presenter (it already runs on the Slint thread).
    assert!(!body.contains("complete_with_usage_endpoint("));
    assert!(!body.contains("invoke_from_event_loop("));
}

#[test]
fn shown_windows_raise_key_front_and_mark_live_covers_startup() {
    let module = read(root(), "src/bin/overlay_host/macos_text_ask.rs");

    // One AppKit adapter serves all three presentation surfaces. Normal
    // post-start shows raise directly after applying the floating behavior.
    assert!(module.contains("slint_replay::native::window::raise_key_front("));
    assert!(module.matches("raise(window.window());").count() >= 5);
    assert!(module.contains("raise(tile.window());"));

    // Windows opened before the event loop become native at mark_live. Reuse
    // that proven boundary for all three instead of maintaining a retry loop.
    let mark_live = module
        .split_once("pub(super) fn mark_live(&self) {")
        .expect("mark_live exists")
        .1
        .split_once("/// Ask chip:")
        .expect("mark_live ends before Ask")
        .0;
    assert_eq!(mark_live.matches("floating(").count(), 3);
    assert_eq!(mark_live.matches("raise(").count(), 3);
    assert!(!module.contains("raise_when_ready"));
}

#[test]
fn setup_window_is_compiled_in_and_translated() {
    let index = read(root(), "ui/index.slint");
    assert!(index.contains("import { MacAiSetupWindow } from \"macos_ai_setup.slint\";"));
    assert!(index.contains("MacAiSetupWindow,"));

    let po = read(root(), "translations/ru/LC_MESSAGES/slint-replay.po");
    for msgid in [
        "Allow mic",
        "Requesting...",
        "Mic denied",
        "Start mic",
        "Mic live",
        "Mic failed",
        "Ask",
        "AI setup",
        "AI bridge setup",
        "Bridge URL",
        "Access token",
        "Paste the access token",
        "Leave empty to keep the saved token",
        "The AI bridge is not configured yet.",
        "The AI bridge is configured.",
        "Check the URLs and access token.",
        "Saved.",
        "Saving the bridge settings failed.",
    ] {
        let entry = format!("msgid \"{msgid}\"");
        assert!(po.contains(&entry), "missing Russian translation: {msgid}");
    }
}

#[test]
fn macos_clipboard_copy_keeps_its_contracts() {
    // The AppKit adapter builds the NSString from bytes + explicit length so
    // an embedded NUL cannot truncate the payload.
    let obj_c = read(root(), "src/native/macos/clipboard.m");
    assert!(obj_c.contains("NSPasteboard"));
    assert!(obj_c.contains("generalPasteboard"));
    assert!(obj_c.contains("initWithBytes"));
    assert!(obj_c.contains("NSUTF8StringEncoding"));
    assert!(obj_c.contains("NSPasteboardTypeString"));
    assert!(obj_c.contains("clearContents"));
    assert!(obj_c.contains("setString"));
    // The C-string convenience APIs stop at the first NUL byte.
    assert!(!obj_c.contains("stringWithUTF8String"));
    assert!(!obj_c.contains("CString"));

    let adapter = read(root(), "src/native/macos/clipboard.rs");
    assert!(adapter.contains("pub fn set_text(text: &str) -> Result<(), String>"));
    assert!(adapter.contains("text.as_ptr()"));
    assert!(adapter.contains("text.len()"));
    // The adapter moves bytes only; it never logs, reads, or clears.
    assert!(!adapter.contains("logging::line"));
    assert!(!adapter.contains("println"));
    assert!(!adapter.contains("read_text"));

    // The adapter is compiled into the existing AppKit build block.
    let build = read(root(), "build.rs");
    assert!(build.contains(".file(\"src/native/macos/clipboard.m\")"));
    assert!(build.contains("cargo:rerun-if-changed=src/native/macos/clipboard.m"));
    let native = read(root(), "src/native/mod.rs");
    assert!(native.contains("#[path = \"macos/clipboard.rs\"]"));

    // Tile wiring reuses the existing hidden full-text property instead of
    // adding a mutex or a second answer store. A new request clears it and
    // hides the main copy affordance; only a finalized answer re-enables it.
    let module = read(root(), "src/bin/overlay_host/macos_text_ask.rs");
    assert!(!module.contains("Mutex<Option<String>>"));
    assert!(module.contains("tile.get_select_text()"));
    assert!(module.contains("tile.set_select_text(SharedString::default())"));
    assert!(module.contains("tile.set_select_text(SharedString::from(answer))"));
    assert!(module.contains("set_can_copy(false)"));
    assert!(module.contains("set_can_copy(true)"));
    assert!(module.contains("on_copy_clicked("));
    assert!(module.contains("on_copy_block_clicked("));
    // Both copy paths flash the existing feedback for exactly 1500 ms.
    assert!(module.contains("const COPY_FEEDBACK_MS: u64 = 1500;"));
    assert!(module.contains("set_copied(true)"));
    assert!(module.contains("set_copied(false)"));
    // The per-code-block flash tracks exactly the callback's block index.
    assert!(module.contains("set_copied_block_index(idx)"));
    assert!(module.contains("set_copied_block_index(-1)"));
    // Clipboard text never reaches a log line; failures stay category-only.
    for line in module.lines() {
        if line.contains("logging::line") {
            for leak in ["{text}", "{code}", "{answer}", "{raw}", "set_text("] {
                assert!(
                    !line.contains(leak),
                    "clipboard payload near a log line: {line}"
                );
            }
        }
    }
}
