//! Phase 0 Day 1 — `slint-replay` pilot binary.
//!
//! Renders the Replay viewer scaffold defined in `ui/replay.slint`. Day 1
//! ships static placeholder data so we can verify the toolchain end-to-end
//! (cargo build, build.rs compile, window spawn, default winit + skia
//! backend on Windows). Day 2 wires the real backend.
//!
//! Run: `cargo run --bin slint-replay` from `slint-experiment/`.

// Slint's generated code uses `unwrap`/`expect`/`panic`/index-slicing
// extensively inside its VTable + property machinery. Our package-level
// `[lints.clippy]` (Tier 3 baseline) forbids those, so we wrap the
// generated code in a module with explicit allows. Our hand-written
// code outside this module still enforces the strict lints.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::pedantic,
    clippy::nursery,
    clippy::all
)]
mod ui {
    slint::include_modules!();
}

use slint::ComponentHandle;
use ui::{FilterChip, MainWindow, ReplayEvent};

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;

    // ----- Day 1 placeholder data -----
    // Replace in Day 2 with results from journal::list_sessions() +
    // journal::load_session().

    let sessions = slint::ModelRc::new(slint::VecModel::from(vec![
        slint::SharedString::from("2026-05-27_14-32-10_abc123.jsonl · 4.2 KB · 2026-05-27 14:32"),
        slint::SharedString::from("2026-05-26_18-05-44_def456.jsonl · 12.8 KB · 2026-05-26 18:05"),
    ]));
    window.set_session_labels(sessions);
    window.set_selected_session_index(0);

    let placeholder_events = vec![
        ReplayEvent {
            kind: "session_start".into(),
            time: "14:32:10".into(),
            label: "SESSION START".into(),
            body: "model=sonnet · prep=haiku · ctx_chars=180 · lang=ru".into(),
            accent: slint::Color::from_rgb_u8(0x34, 0xd3, 0x99),
        },
        ReplayEvent {
            kind: "transcript_line".into(),
            time: "14:32:15".into(),
            label: "🎤 mic".into(),
            body: "пример строки транскрипта (Day 1 placeholder)".into(),
            accent: slint::Color::from_rgb_u8(0x6b, 0x72, 0x80),
        },
        ReplayEvent {
            kind: "detector_decision".into(),
            time: "14:32:18".into(),
            label: "DETECT ✓".into(),
            body: "→ question (keyword: kubernetes)".into(),
            accent: slint::Color::from_rgb_u8(0x34, 0xd3, 0x99),
        },
        ReplayEvent {
            kind: "ai_request".into(),
            time: "14:32:19".into(),
            label: "AI REQ · live_ask".into(),
            body: "claude-sonnet-4-6 · ~240 in-tok · placeholder prompt".into(),
            accent: slint::Color::from_rgb_u8(0x81, 0x8c, 0xf8),
        },
        ReplayEvent {
            kind: "ai_response".into(),
            time: "14:32:22".into(),
            label: "AI RESP · live_ask".into(),
            body: "2840 ms · finish=stop · $0.0034 · placeholder answer".into(),
            accent: slint::Color::from_rgb_u8(0x81, 0x8c, 0xf8),
        },
        ReplayEvent {
            kind: "tile_spawn".into(),
            time: "14:32:23".into(),
            label: "TILE".into(),
            body: "What is kubernetes? · A container orchestration platform...".into(),
            accent: slint::Color::from_rgb_u8(0xf4, 0x72, 0xb6),
        },
    ];

    let chips = vec![
        FilterChip {
            kind: "transcript_line".into(),
            count: 1,
            hidden: false,
            accent: slint::Color::from_rgb_u8(0x6b, 0x72, 0x80),
        },
        FilterChip {
            kind: "ai_response".into(),
            count: 1,
            hidden: false,
            accent: slint::Color::from_rgb_u8(0x81, 0x8c, 0xf8),
        },
        FilterChip {
            kind: "session_start".into(),
            count: 1,
            hidden: false,
            accent: slint::Color::from_rgb_u8(0x34, 0xd3, 0x99),
        },
    ];

    window.set_events(slint::ModelRc::new(slint::VecModel::from(placeholder_events)));
    window.set_filter_chips(slint::ModelRc::new(slint::VecModel::from(chips)));
    window.set_total_events(6);
    window.set_ai_response_count(1);
    window.set_total_cost_display("0.0034".into());

    // ----- Callbacks (Day 1 stubs — Day 2 wires real handlers) -----

    window.on_session_changed(|idx| {
        eprintln!("[slint-replay] session-changed: index={idx}");
    });

    window.on_chip_clicked(|idx| {
        eprintln!("[slint-replay] chip-clicked: index={idx}");
    });

    window.on_reset_filter(|| {
        eprintln!("[slint-replay] reset-filter");
    });

    let weak = window.as_weak();
    window.on_back_clicked(move || {
        if let Some(w) = weak.upgrade() {
            // For the pilot we just close the window. Real Replay navigates
            // back to the overlay via `window.location.search = ""` (React) —
            // in Slint this would be hiding the Replay window and showing
            // the overlay window, handled by Phase 1's window manager.
            let _ = w.hide();
        }
    });

    window.run()
}
