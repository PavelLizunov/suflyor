//! Keep fixed capsule geometry and native window sizing on one Slint contract.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn expression<'a>(source: &'a str, property: &str) -> &'a str {
    source
        .split_once(&format!("out property <length> {property}:"))
        .unwrap_or_else(|| panic!("missing BarSpec property: {property}"))
        .1
        .split_once(';')
        .expect("terminated size expression")
        .0
}

#[test]
fn bar_spec_exists_and_widths_sum_every_region() {
    let spec = read("ui/bar_spec.slint");
    assert!(spec.contains("export global BarSpec {"));
    for layout in ["full", "compact"] {
        let total = expression(&spec, &format!("{layout}-width"));
        assert!(total.contains('+'), "total must be a sum, not a literal");
        assert!(!total.contains("px"), "total must use named dimensions");
        for part in ["badge", "pad", "gap", "spring"] {
            assert!(total.contains(part), "{layout} omits {part}");
            expression(&spec, part);
        }
        for region in ["state", "voice", "session", "tiles", "window"] {
            let property = format!("{layout}-{region}");
            assert!(total.contains(&property), "{layout} total omits {property}");
            expression(&spec, &property);
        }
        assert!(total.contains("3 * pad"));
        assert!(total.contains("5 * gap"));
    }
    assert_eq!(expression(&spec, "height").trim(), "64px");
    let index = read("ui/index.slint");
    assert!(index.contains("import { BarSpec } from \"bar_spec.slint\";"));
    assert!(index.contains("    BarSpec,"));
}

#[test]
fn bar_colours_are_tokens_outside_the_unchanged_bootstrap_section() {
    let bar = read("ui/overlay_bar.slint");
    let normal = bar.split_once("// First honest production surface").expect("bootstrap boundary").0;
    for (line, code) in normal.lines().enumerate() {
        let code = code.split("//").next().unwrap_or("");
        let bytes = code.as_bytes();
        for (offset, byte) in bytes.iter().enumerate() {
            if *byte == b'#' {
                assert!(bytes[offset + 1..].iter().take_while(|byte| byte.is_ascii_hexdigit()).count() < 6,
                    "raw hex colour in overlay_bar.slint:{}", line + 1);
            }
        }
    }
}

#[test]
fn native_size_and_capsule_fit_evidence_use_the_spec() {
    let host = read("src/bin/overlay_host/bar_tray.rs");
    assert!(!host.contains("1280.0"));
    assert!(!host.contains("680.0"));
    let sizing = host.split_once("fn apply_bar_size(").expect("sizing helper").1
        .split_once("fn recenter_when_sized(").expect("next helper").0;
    assert!(!sizing.contains("64.0"));
    assert!(sizing.contains("overlay.global::<crate::ui::BarSpec>()"));
    for getter in ["get_full_width()", "get_compact_width()", "get_height()"] {
        assert!(sizing.contains(getter), "missing size getter {getter}");
    }
    let bar = read("ui/overlay_bar.slint");
    assert!(bar.contains("preferred-width: root.bootstrap-mode ? 560px : (root.compact-bar ? BarSpec.compact-width : BarSpec.full-width);"));
    assert!(bar.contains("preferred-height: BarSpec.height;"));
    assert!(bar.contains("min-width: root.bootstrap-mode ? 440px : (root.compact-bar ? BarSpec.compact-width : BarSpec.full-width);"));
    assert!(bar.contains("out property <bool> fits: lay.preferred-width <= self.width;"));
    let fit = bar.split_once("out property <bool> all-regions-fit:").expect("fit accessor").1
        .split_once(';').expect("terminated fit binding").0;
    for region in ["state", "voice", "session", "tiles", "window"] {
        assert!(fit.contains(&format!("{region}-capsule.fits")));
        assert!(bar.contains(&format!("{region}-capsule := Capsule {{")));
        assert!(bar.contains(&format!("width: root.compact-bar ? BarSpec.compact-{region} : BarSpec.full-{region};")));
    }
}
