//! Guard the complete Astra SVG icon set and its production contract.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

const ROOT_OPEN: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"#ffffff\" stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\">";

const ICONS: [&str; 50] = [
    "ai.svg",
    "archive.svg",
    "arrow-up.svg",
    "audio.svg",
    "brain.svg",
    "bubble.svg",
    "camera.svg",
    "check.svg",
    "coach.svg",
    "copy.svg",
    "diag.svg",
    "flame.svg",
    "gear.svg",
    "grip.svg",
    "kb.svg",
    "knowledge.svg",
    "lock.svg",
    "maximize.svg",
    "mic.svg",
    "micslash.svg",
    "minimize.svg",
    "monitor.svg",
    "more.svg",
    "pause.svg",
    "pencil.svg",
    "play.svg",
    "plus.svg",
    "refresh.svg",
    "restart.svg",
    "restore.svg",
    "seek-back.svg",
    "seek-forward.svg",
    "select-text.svg",
    "session-start.svg",
    "sos.svg",
    "speed.svg",
    "star.svg",
    "stealth.svg",
    "stop.svg",
    "stt.svg",
    "summary.svg",
    "system-audio.svg",
    "tiles.svg",
    "trash.svg",
    "tray.svg",
    "unlock.svg",
    "update.svg",
    "user.svg",
    "voice.svg",
    "x.svg",
];

fn allowed_attributes(tag: &str) -> Option<&'static [&'static str]> {
    match tag {
        "path" => Some(&["d"]),
        "rect" => Some(&["x", "y", "width", "height", "rx", "ry"]),
        "circle" => Some(&["cx", "cy", "r"]),
        "line" => Some(&["x1", "y1", "x2", "y2"]),
        "polyline" | "polygon" => Some(&["points"]),
        "ellipse" => Some(&["cx", "cy", "rx", "ry"]),
        _ => None,
    }
}

fn validate_primitive(name: &str, source: &str) -> Result<(), String> {
    let source = source.strip_suffix('/').unwrap_or(source).trim();
    let split = source.find(char::is_whitespace).unwrap_or(source.len());
    let tag = &source[..split];
    let Some(allowed) = allowed_attributes(tag) else {
        return Err(format!("{name}: element <{tag}> is forbidden"));
    };

    let mut rest = source[split..].trim();
    if rest.is_empty() {
        return Err(format!("{name}: <{tag}> has no geometry"));
    }
    while !rest.is_empty() {
        let equals = rest
            .find('=')
            .ok_or_else(|| format!("{name}: malformed <{tag}> attribute"))?;
        let attribute = rest[..equals].trim();
        if !allowed.contains(&attribute) {
            return Err(format!(
                "{name}: attribute {attribute} is forbidden on <{tag}>"
            ));
        }
        rest = rest[equals + 1..].trim_start();
        if !rest.starts_with('"') {
            return Err(format!(
                "{name}: attribute {attribute} must use double quotes"
            ));
        }
        rest = &rest[1..];
        let quote = rest
            .find('"')
            .ok_or_else(|| format!("{name}: unterminated {attribute} value"))?;
        if rest[..quote].is_empty() {
            return Err(format!("{name}: attribute {attribute} is empty"));
        }
        rest = rest[quote + 1..].trim_start();
    }
    Ok(())
}

fn validate_icon(name: &str, svg: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(body) = svg
        .trim()
        .strip_prefix(ROOT_OPEN)
        .and_then(|source| source.strip_suffix("</svg>"))
    else {
        return vec![format!(
            "{name}: root must exactly match the Astra 16x16 outline contract"
        )];
    };

    let mut rest = body.trim();
    let mut primitives = 0;
    while !rest.is_empty() {
        if !rest.starts_with('<') {
            failures.push(format!(
                "{name}: text outside an SVG primitive is forbidden"
            ));
            break;
        }
        let Some(end) = rest.find('>') else {
            failures.push(format!("{name}: unterminated SVG element"));
            break;
        };
        let element = &rest[1..end];
        if !element.ends_with('/') {
            failures.push(format!("{name}: only self-closing primitives are allowed"));
            break;
        }
        if let Err(failure) = validate_primitive(name, element) {
            failures.push(failure);
        }
        primitives += 1;
        rest = rest[end + 1..].trim_start();
    }
    if primitives == 0 {
        failures.push(format!("{name}: icon must contain a vector primitive"));
    }
    failures
}

#[test]
fn complete_icon_set_uses_the_astra_contract() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
    let mut actual: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .map(|entry| entry.expect("read icon entry").file_name())
        .filter_map(|name| name.into_string().ok())
        .filter(|name| name.ends_with(".svg"))
        .collect();
    actual.sort();
    assert_eq!(actual, ICONS, "Astra icon filenames changed");

    let mut failures = Vec::new();
    for name in ICONS {
        let path = dir.join(name);
        let svg = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        failures.extend(validate_icon(name, &svg));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn rejects_non_astra_svg_features() {
    let root = "<svg data-fill=\"none\"><path d=\"M2 2h12\"/></svg>".to_owned();
    let quotes = format!("{ROOT_OPEN}<path d='M2 2h12'/></svg>");
    let group = format!("{ROOT_OPEN}<g><path d=\"M2 2h12\"/></g></svg>");
    let override_style = format!("{ROOT_OPEN}<path d=\"M2 2h12\" stroke=\"none\"/></svg>");
    let external = format!("{ROOT_OPEN}<use href=\"icon.svg\"/></svg>");
    for (label, svg) in [
        ("root", root.as_str()),
        ("quotes", quotes.as_str()),
        ("group", group.as_str()),
        ("override", override_style.as_str()),
        ("external", external.as_str()),
    ] {
        assert!(
            !validate_icon(label, svg).is_empty(),
            "invalid fixture passed: {label}"
        );
    }
}
