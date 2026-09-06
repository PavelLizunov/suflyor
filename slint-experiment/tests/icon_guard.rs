//! Guard the shared SVG grid and Astra icon convention.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

const ROOT_ATTRIBUTES: [(&str, &str); 6] = [
    ("viewBox", "0 0 16 16"),
    ("fill", "none"),
    ("stroke", "#ffffff"),
    ("stroke-width", "1.6"),
    ("stroke-linecap", "round"),
    ("stroke-linejoin", "round"),
];

const FORBIDDEN: [&str; 12] = [
    "<filter",
    "<lineargradient",
    "<radialgradient",
    "<text",
    "<tspan",
    "<style",
    "<script",
    "<mask",
    "filter=",
    "style=",
    "transform=",
    "data:",
];

fn attribute_values<'a>(source: &'a str, attribute: &str) -> Vec<&'a str> {
    let marker = format!("{attribute}=\"");
    source
        .match_indices(&marker)
        .filter_map(|(start, _)| source[start + marker.len()..].split('"').next())
        .collect()
}

fn validate_icon(name: &str, svg: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let lower = svg.to_ascii_lowercase();
    let root = svg
        .find("<svg")
        .and_then(|start| svg[start..].find('>').map(|end| &svg[start..start + end]));

    let Some(root) = root else {
        return vec![format!("{name}: missing root <svg> element")];
    };

    for (attribute, value) in ROOT_ATTRIBUTES {
        if !root.contains(&format!("{attribute}=\"{value}\"")) {
            failures.push(format!("{name}: root {attribute} must be {value}"));
        }
    }

    for forbidden in FORBIDDEN {
        if lower.contains(forbidden) {
            failures.push(format!("{name}: forbidden SVG construct {forbidden}"));
        }
    }

    for value in attribute_values(svg, "stroke-width") {
        if value != "1.6" {
            failures.push(format!("{name}: stroke-width must be 1.6"));
        }
    }
    for attribute in ["fill", "stroke", "color"] {
        for value in attribute_values(svg, attribute) {
            if !matches!(value.to_ascii_lowercase().as_str(), "none" | "#ffffff") {
                failures.push(format!(
                    "{name}: {attribute} color must be none or #ffffff, found {value}"
                ));
            }
        }
    }
    if lower.contains("rgb(") || lower.contains("rgba(") || lower.contains("hsl(") {
        failures.push(format!("{name}: color functions are forbidden"));
    }

    let has_primitive = [
        "<path",
        "<rect",
        "<circle",
        "<line",
        "<polyline",
        "<polygon",
        "<ellipse",
    ]
    .iter()
    .any(|primitive| lower.contains(primitive));
    if !has_primitive {
        failures.push(format!(
            "{name}: icon must contain a visible vector primitive"
        ));
    }

    failures
}

#[test]
fn every_icon_uses_the_astra_contract() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .map(|entry| entry.expect("read icon entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("svg"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no SVG icons found in {}", dir.display());

    let mut failures = Vec::new();
    for path in paths {
        let svg = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        failures.extend(validate_icon(&name, &svg));
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn rejects_non_astra_svg_features() {
    let invalid = r##"<svg viewBox="0 0 16 16" fill="none" stroke="#ffffff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><filter id="glow"/><path d="M2 2h12" stroke="#ff0000"/></svg>"##;
    let failures = validate_icon("invalid.svg", invalid).join("\n");
    assert!(failures.contains("stroke-width"));
    assert!(failures.contains("filter"));
    assert!(failures.contains("#ff0000"));
}
