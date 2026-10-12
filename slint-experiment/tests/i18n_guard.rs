//! i18n drift guard (added 2026-06-13 during the UI-audit methodology pass).
//!
//! Catches the two classes that kept reaching the user: a bare string literal
//! in a Slint `text` / `placeholder-text` / `title` / `accessible-label`
//! expression, or an `@tr("English…")` string with NO matching `msgid` in the
//! Russian `.po`. clippy/cargo-test were blind to both — now they aren't.
//! Pure file parsing, no UI build needed.
//!
//! If this fails: either add the `msgid`/`msgstr` pair to
//! `translations/ru/LC_MESSAGES/slint-replay.po`, or (rarely) the string is a
//! deliberate non-translatable token — then it shouldn't be wrapped in `@tr`.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test asserts

use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Extract the first string-literal argument of every `@tr("…")` in `src`.
/// Mirrors Slint's own scan: only the leading literal is the msgid (format
/// args after a comma are ignored). Handles escaped quotes `\"`.
fn tr_msgids(src: &str) -> Vec<String> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let needle = b"@tr(";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            // skip whitespace to the opening quote
            let mut j = i + needle.len();
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                j += 1;
                let mut s = String::new();
                while j < bytes.len() {
                    match bytes[j] {
                        b'\\' if j + 1 < bytes.len() => {
                            // keep the escape sequence verbatim (matches .po form)
                            s.push('\\');
                            s.push(bytes[j + 1] as char);
                            j += 2;
                        }
                        b'"' => break,
                        c => {
                            // push raw byte; rebuild utf-8 below via from_utf8 of slice
                            s.push(c as char);
                            j += 1;
                        }
                    }
                }
                out.push(s);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Pull every `msgid "…"` from a `.po` file.
fn po_msgids(src: &str) -> HashSet<String> {
    src.lines()
        .filter_map(|l| {
            let l = l.trim_start();
            let rest = l.strip_prefix("msgid ")?;
            let rest = rest.trim();
            let inner = rest.strip_prefix('"')?.strip_suffix('"')?;
            Some(inner.to_string())
        })
        .collect()
}

/// Return `text` / `placeholder-text` / `title` / `accessible-label`
/// expressions with their source line. Window titles (taskbar / alt-tab) and
/// accessible labels (screen readers) are user-facing too. Struct field
/// declarations (`title: string,`) end in `,` and are NOT collected.
/// This is deliberately a tiny Slint-shaped scanner, not a general parser:
/// property expressions end at the first non-string `;`.
fn text_expressions(src: &str) -> Vec<(usize, &str)> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;

    while i < bytes.len() {
        if bytes[i] == b'\n' {
            line += 1;
            i += 1;
            continue;
        }
        if bytes[i..].starts_with(b"//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i..].starts_with(b"/*") {
            i += 2;
            while i + 1 < bytes.len() && !bytes[i..].starts_with(b"*/") {
                if bytes[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    if bytes[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
            continue;
        }

        let previous_is_ident =
            i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || matches!(bytes[i - 1], b'_' | b'-'));
        let property_len = [
            b"placeholder-text".as_slice(),
            b"accessible-label".as_slice(),
            b"title".as_slice(),
            b"text".as_slice(),
        ]
        .into_iter()
        .find_map(|name| {
            (!previous_is_ident && bytes[i..].starts_with(name)).then_some(name.len())
        });
        let Some(property_len) = property_len else {
            i += 1;
            continue;
        };
        let mut start = i + property_len;
        while start < bytes.len() && bytes[start].is_ascii_whitespace() {
            start += 1;
        }
        if bytes.get(start) != Some(&b':') {
            i += 1;
            continue;
        }
        start += 1;

        let expression_line = line;
        let mut end = start;
        let mut depth = 0usize;
        let mut terminated = false;
        while end < bytes.len() {
            if bytes[end] == b'"' {
                end += 1;
                while end < bytes.len() {
                    if bytes[end] == b'\\' && end + 1 < bytes.len() {
                        end += 2;
                    } else if bytes[end] == b'"' {
                        end += 1;
                        break;
                    } else {
                        end += 1;
                    }
                }
            } else {
                match bytes[end] {
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => depth = depth.saturating_sub(1),
                    b',' if depth == 0 => break,
                    b';' if depth == 0 => {
                        terminated = true;
                        break;
                    }
                    _ => {}
                }
                end += 1;
            }
        }
        if terminated {
            out.push((expression_line, &src[start..end]));
        }
        line += bytes[i..end.min(bytes.len())]
            .iter()
            .filter(|&&b| b == b'\n')
            .count();
        i = (end + 1).min(bytes.len());
    }
    out
}

fn visible_literal_text(literal: &str) -> String {
    let mut out = String::new();
    let mut chars = literal.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'{') {
            let _ = chars.next();
            for inner in chars.by_ref() {
                if inner == '}' {
                    break;
                }
            }
        } else if ch == '\\' {
            if let Some(escaped) = chars.next() {
                if !matches!(escaped, 'n' | 'r' | 't') {
                    out.push(escaped);
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn needs_translation(literal: &str) -> bool {
    let visible = visible_literal_text(literal);
    let visible = visible.trim();
    if visible.is_empty()
        || matches!(visible, "AI" | "MLX" | "STT" | "px")
        || visible.starts_with("http://")
        || visible.starts_with("https://")
    {
        return false;
    }
    visible.chars().any(char::is_alphabetic)
}

fn bare_text_literals(expression: &str) -> Vec<String> {
    let bytes = expression.as_bytes();
    let mut out = Vec::new();
    let mut parens = Vec::new();
    let mut tr_depth = 0;
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'(' => {
                let is_tr = expression[..i].trim_end().ends_with("@tr");
                parens.push(is_tr);
                tr_depth += usize::from(is_tr);
                i += 1;
            }
            b')' => {
                if parens.pop() == Some(true) {
                    tr_depth -= 1;
                }
                i += 1;
            }
            b'"' => {
                let start = i;
                i += 1;
                let literal_start = i;
                while i < bytes.len() {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                    } else if bytes[i] == b'"' {
                        break;
                    } else {
                        i += 1;
                    }
                }
                let literal = &expression[literal_start..i.min(bytes.len())];
                let before = expression[..start].trim_end();
                let after = expression[(i + 1).min(bytes.len())..].trim_start();
                let comparison_operand = before.ends_with("==")
                    || before.ends_with("!=")
                    || after.starts_with("==")
                    || after.starts_with("!=");
                if tr_depth == 0 && !comparison_operand && needs_translation(literal) {
                    out.push(literal.to_string());
                }
                i = (i + 1).min(bytes.len());
            }
            _ => i += 1,
        }
    }
    out
}

#[test]
fn bare_literal_scanner_distinguishes_display_text_from_tokens() {
    let src = r#"
        Text { text: "Replay"; }
        Text { text: root.count + " events"; }
        Text {
            text: root.busy
                ? "Send"
                : @tr("Ready");
        }
        Text { text: root.mode == "queue" ? @tr("Clear queue") : ""; }
        Text { text: root.busy ? "…" : "→"; }
        Text { text: "AI"; }
        Text { text: "MLX"; }
        Text { text: "127.0.0.1"; }
        Text { text: "\{root.pos} / \{root.count}"; }
        Window { title: "Session archive"; }
        Window { title: @tr("Help"); }
        Window { title: root.dynamic-title; }
        Rectangle { accessible-label: "Retry"; }
        Rectangle { accessible-label: @tr("Close"); }
        struct S { title: string, }
    "#;
    let found: Vec<String> = text_expressions(src)
        .into_iter()
        .flat_map(|(_, expression)| bare_text_literals(expression))
        .collect();
    assert_eq!(
        found,
        ["Replay", " events", "Send", "Session archive", "Retry"]
    );
}

#[test]
fn every_display_literal_is_translated_or_technical() {
    const EXCLUDED_DEV_HARNESSES: [&str; 2] = ["markdown_spike.slint", "overlay_spike.slint"];

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut bare: Vec<(String, usize, String)> = Vec::new();
    for entry in fs::read_dir(root.join("ui")).expect("read ui dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("slint") {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        if EXCLUDED_DEV_HARNESSES.contains(&name) {
            continue;
        }
        let src = fs::read_to_string(&path).expect("read slint");
        for (line, expression) in text_expressions(&src) {
            for literal in bare_text_literals(expression) {
                bare.push((name.to_string(), line, literal));
            }
        }
    }
    assert!(
        bare.is_empty(),
        "bare user-facing Slint strings — wrap in @tr and add msgid/msgstr:\n{}",
        bare.iter()
            .map(|(file, line, literal)| format!("  [{file}:{line}] \"{literal}\""))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_tr_string_has_a_russian_translation() {
    // The byte-level scan above mangles multi-byte UTF-8 (pushes each byte as a
    // char). That's fine for ASCII msgids — and Slint @tr msgids are the ENGLISH
    // source, i.e. ASCII — so any string containing non-ASCII is a Cyrillic
    // literal we skip here (the .po side stores the English msgid, not Cyrillic).
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let po = fs::read_to_string(root.join("translations/ru/LC_MESSAGES/slint-replay.po"))
        .expect("read ru .po");
    let msgids = po_msgids(&po);

    let ui_dir = root.join("ui");
    let mut missing: Vec<(String, String)> = Vec::new();
    for entry in fs::read_dir(&ui_dir).expect("read ui dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("slint") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let src = fs::read_to_string(&path).expect("read slint");
        for id in tr_msgids(&src) {
            // Only ASCII msgids are real English source strings that MUST be in
            // the .po; a mangled multi-byte string isn't a clean key.
            if !id.is_ascii() || id.is_empty() {
                continue;
            }
            if !msgids.contains(&id) {
                missing.push((name.clone(), id));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "@tr strings with NO Russian translation (RU users see English) — \
         add msgid/msgstr to slint-replay.po:\n{}",
        missing
            .iter()
            .map(|(f, s)| format!("  [{f}] \"{s}\""))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
