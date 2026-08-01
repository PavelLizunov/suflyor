//! Guard: every production `Command::new` in overlay-backend must apply
//! `CREATE_NO_WINDOW` (via the shared `download::no_window` helper, the
//! `hidden_command` wrapper, or an inline `creation_flags` call) so
//! GUI-launched workers never flash a console window on Windows.
//!
//! Mirrors the source-scan approach of `slint-experiment/tests/icon_guard.rs`.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

/// Production source files that spawn external processes.
const SPAWN_FILES: &[&str] = &[
    "src/download.rs",
    "src/local_ai.rs",
    "src/tts.rs",
    "src/ocr.rs",
    "src/diarize.rs",
    "src/update.rs",
];

/// Substrings that mark a `Command::new` site as intentionally windowless.
const HIDDEN_MARKERS: &[&str] = &["no_window", "hidden_command", "creation_flags"];

/// `Command::new` targets that are GUI applications — no console to hide.
/// Matched against the ±12-line context (function names, comments, etc.).
const GUI_ALLOWLIST: &[&str] = &["explorer", "run_installer"];

#[test]
fn every_command_spawn_hides_its_console() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut failures = Vec::new();

    for file in SPAWN_FILES {
        let path = root.join(file);
        let src = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("read {file}: {e}");
        });
        // Only scan production code — stop at the first #[cfg(test)] module.
        let prod = src.split("#[cfg(test)]").next().unwrap_or(&src);
        let lines: Vec<&str> = prod.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if !line.contains("Command::new") {
                continue;
            }
            // Check a ±12-line window for a hidden-console marker or a
            // known GUI-app spawn (function name / comment in context).
            let start = i.saturating_sub(12);
            let end = (i + 13).min(lines.len());
            let context: String = lines[start..end].join("\n");
            let is_gui = GUI_ALLOWLIST.iter().any(|g| context.contains(g));
            if is_gui {
                continue;
            }
            let has_marker = HIDDEN_MARKERS.iter().any(|m| context.contains(m));
            if !has_marker {
                failures.push(format!(
                    "{file}:{}: Command::new without CREATE_NO_WINDOW",
                    i + 1
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "console-window guard failed:\n{}",
        failures.join("\n")
    );
}
