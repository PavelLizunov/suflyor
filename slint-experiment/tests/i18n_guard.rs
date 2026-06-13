//! i18n drift guard (added 2026-06-13 during the UI-audit methodology pass).
//!
//! Catches the exact class that kept reaching the user: a `@tr("English…")`
//! string in a `.slint` file with NO matching `msgid` in the Russian `.po`, so
//! a RU user sees the English fallback. clippy/cargo-test were blind to this —
//! now they aren't. Pure file parsing, no UI build needed.
//!
//! If this fails: either add the `msgid`/`msgstr` pair to
//! `translations/ru/LC_MESSAGES/slint-replay.po`, or (rarely) the string is a
//! deliberate non-translatable token — then it shouldn't be wrapped in `@tr`.

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
