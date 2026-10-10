//! Design rules that a build can check without opening a window.
//!
//! 1. A window that uses std-widgets sets their palette from the theme, so a
//!    field or a button never shows the light default on a dark window.
//! 2. The host does not build colours in Rust: it names a tone and the theme
//!    picks the colour for the active scheme. The one exception is the
//!    per-speaker palette of the transcript window.
//! 3. Sizes and colours come from `metrics.slint` and `theme.slint`. The
//!    literals that still bypass them are counted, and the count may only go
//!    down: lower the limit in the same change that removes a literal.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test asserts

use std::fs;
use std::path::{Path, PathBuf};

/// Font sizes written as a number instead of a `Metrics.font-*` token.
const FONT_SIZE_LITERALS: usize = 18;
/// Corner radii written as a number instead of a `Metrics.radius-*` token.
const RADIUS_LITERALS: usize = 71;
/// Hex colours outside `theme.slint`.
const HEX_COLOURS: usize = 47;

/// Not part of the shipped overlay: two layout spikes and the replay viewer.
const NOT_SHIPPED: &[&str] = &["markdown_spike.slint", "overlay_spike.slint", "replay.slint"];

fn ui_files() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .expect("ui directory")
        .map(|entry| entry.expect("ui entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "slint"))
        .collect();
    files.sort();
    files
}

fn name(path: &Path) -> String {
    path.file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned()
}

/// The line without a trailing `//` comment.
fn code(line: &str) -> &str {
    line.find("//").map_or(line, |at| &line[..at])
}

/// How many times `key` is followed by a number on this line.
fn literal_after(code: &str, key: &str) -> usize {
    code.match_indices(key)
        .filter(|(at, _)| {
            code[at + key.len()..]
                .trim_start()
                .starts_with(|c: char| c.is_ascii_digit())
        })
        .count()
}

/// How many `#` are followed by at least three hex digits on this line.
fn hex_colours(code: &str) -> usize {
    code.match_indices('#')
        .filter(|(at, _)| {
            code[at + 1..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .count()
                >= 3
        })
        .count()
}

#[test]
fn a_window_with_std_widgets_takes_their_palette_from_the_theme() {
    let mut missing = Vec::new();
    for path in ui_files() {
        let file = name(&path);
        if NOT_SHIPPED.contains(&file.as_str()) {
            continue;
        }
        let src = fs::read_to_string(&path).expect("read ui file");
        let is_window = src.contains(" inherits Window {");
        let uses_widgets = src.contains("\"std-widgets.slint\"");
        if is_window && uses_widgets && !src.contains("Palette.color-scheme") {
            missing.push(file);
        }
    }
    assert!(
        missing.is_empty(),
        "these windows use std-widgets and do not set Palette.color-scheme from Theme.is-light: {missing:?}"
    );
}

#[test]
fn the_host_does_not_build_colours_in_rust() {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("source directory") {
            let path = entry.expect("source entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("bin");
    let mut files = Vec::new();
    walk(&root.join("overlay_host"), &mut files);
    files.push(root.join("overlay_host_windows.rs"));
    let mut found = Vec::new();
    for path in files {
        // The transcript window gives each speaker a colour of their own.
        if path.ends_with("aux_windows/transcript.rs") {
            continue;
        }
        let src = fs::read_to_string(&path).expect("read source file");
        for (index, line) in src.lines().enumerate() {
            let code = code(line);
            if code.contains("Color::from_rgb") || code.contains("Color::from_argb") {
                found.push(format!("{}:{}", name(&path), index + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "name a tone (StatusTone, TileTone) and let theme.slint pick the colour: {found:?}"
    );
}

#[test]
fn literals_that_bypass_the_tokens_do_not_grow() {
    let (mut font, mut radius, mut hex) = (0, 0, 0);
    for path in ui_files() {
        let file = name(&path);
        if file == "theme.slint" || file == "metrics.slint" || NOT_SHIPPED.contains(&file.as_str()) {
            continue;
        }
        let src = fs::read_to_string(&path).expect("read ui file");
        for line in src.lines() {
            let code = code(line);
            font += literal_after(code, "font-size:");
            radius += literal_after(code, "border-radius:");
            hex += hex_colours(code);
        }
    }
    assert!(
        font <= FONT_SIZE_LITERALS,
        "{font} font-size literals, the limit is {FONT_SIZE_LITERALS}: use a Metrics.font-* token"
    );
    assert!(
        radius <= RADIUS_LITERALS,
        "{radius} border-radius literals, the limit is {RADIUS_LITERALS}: use a Metrics.radius-* token"
    );
    assert!(
        hex <= HEX_COLOURS,
        "{hex} hex colours outside theme.slint, the limit is {HEX_COLOURS}: use a Theme token"
    );
}
