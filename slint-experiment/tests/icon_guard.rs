//! Guard the shared SVG grid and stroke convention.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

#[test]
fn every_icon_uses_the_shared_grid_and_stroke() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
    let mut failures = Vec::new();
    for entry in fs::read_dir(dir).expect("read icon directory") {
        let path = entry.expect("read icon entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("svg") {
            continue;
        }
        let svg = fs::read_to_string(&path).expect("read icon");
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !svg.contains("viewBox=\"0 0 16 16\"") {
            failures.push(format!("{name}: viewBox must be 0 0 16 16"));
        }
        for value in svg.split("stroke-width=\"").skip(1) {
            if value.split('"').next() != Some("1.6") {
                failures.push(format!("{name}: stroke-width must be 1.6"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
