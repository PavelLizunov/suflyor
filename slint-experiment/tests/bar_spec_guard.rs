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
    assert_eq!(
        expression(&spec, "full-tiles").trim(),
        "96px + 92px + 22px + 26px + 3 * 2px + 2 * 2px"
    );
    assert_eq!(expression(&spec, "compact-tiles").trim(), "26px + 2 * 2px");
    let mini = expression(&spec, "mini-width");
    assert!(mini.contains('+'), "mini total must be a sum");
    assert!(!mini.contains("px"), "mini total must not be literal");
    for part in ["badge", "pad", "gap", "spring"] {
        assert!(mini.contains(part), "mini omits {part}");
    }
    for region in ["voice", "session", "tiles", "window"] {
        let name = format!("mini-{region}");
        assert!(mini.contains(&name), "mini total omits {name}");
        expression(&spec, &name);
    }
    assert!(!mini.contains("state"), "mini has four capsules, not five");
    assert!(mini.contains("3 * pad"));
    assert!(mini.contains("4 * gap"));
    let index = read("ui/index.slint");
    assert!(index.contains("import { BarSpec } from \"bar_spec.slint\";"));
    assert!(index.contains("    BarSpec,"));
}

#[test]
fn monitor_sampling_and_recovery_keep_native_work_area_and_reveal_contracts() {
    let host = read("src/bin/overlay_host/bar_tray.rs");
    let main = read("src/bin/overlay_host_windows.rs");
    assert!(host.contains("work_area_for_window(hwnd)"));
    assert!(host.contains("work.width() as f32 / overlay.window().scale_factor().max(0.1)"));
    assert!(
        host.contains("x != -32000 || y != -32000"),
        "parked startup uses primary"
    );
    assert!(host.contains("work_area_for_point(m.left + m.width() / 2, m.top + m.height() / 2)"));
    assert!(
        host.contains("let monitor_width = f32::INFINITY;"),
        "non-Windows keeps preference-only selection"
    );
    assert!(host.contains("const BAR_MARGIN: f32 = 20.0;"));
    assert!(host.contains("x + width <= work.right && y + height <= work.bottom"));
    assert!(host.contains("if contained && form != BarForm::TooNarrow"));
    assert!(host.contains("work_area_for_window(hwnd).or_else(primary_bar_work_area)"));
    assert!(host.contains("work.left + (BAR_MARGIN * scale).round() as i32"));
    assert!(host.contains("(work.top + 24).clamp(work.top, (work.bottom - height).max(work.top))"));
    assert!(
        host.contains("if selected_width != target_w_logical"),
        "stale deferred resize must not reposition newer form"
    );
    assert!(
        host.contains("move_window_pos_only(hwnd, 60, 24)"),
        "hard native reveal fallback remains"
    );
    assert!(host.contains("realize_with_retries(overlay, attempt, fallback)"));
    assert!(host.contains("set_global_stealth_effective(false)"));
    assert!(main.contains("last_bar_monitor_check.get().elapsed() >= Duration::from_secs(2)"));
    let tick = main
        .split_once("last_bar_monitor_check.set(std::time::Instant::now());")
        .unwrap()
        .1;
    assert!(
        tick.split_once("let (active, secs)")
            .unwrap()
            .0
            .contains("reevaluate_bar_monitor(&o)"),
        "monitor check runs even with no active session"
    );
    assert!(main.contains("c.compact_bar = if was_mini { false } else { !c.compact_bar };"));
    assert!(main.contains("bar expand requested: monitor still requires mini"));
}

#[test]
fn bar_colours_are_tokens_outside_the_unchanged_bootstrap_section() {
    let bar = read("ui/overlay_bar.slint");
    let normal = bar
        .split_once("// First honest production surface")
        .expect("bootstrap boundary")
        .0;
    for (line, code) in normal.lines().enumerate() {
        let code = code.split("//").next().unwrap_or("");
        let bytes = code.as_bytes();
        for (offset, byte) in bytes.iter().enumerate() {
            if *byte == b'#' {
                assert!(
                    bytes[offset + 1..]
                        .iter()
                        .take_while(|byte| byte.is_ascii_hexdigit())
                        .count()
                        < 6,
                    "raw hex colour in overlay_bar.slint:{}",
                    line + 1
                );
            }
        }
    }
}

#[test]
fn native_size_and_capsule_fit_evidence_use_the_spec() {
    let host = read("src/bin/overlay_host/bar_tray.rs");
    assert!(!host.contains("1280.0"));
    assert!(!host.contains("680.0"));
    let sizing = host
        .split_once("fn apply_bar_size(")
        .expect("sizing helper")
        .1
        .split_once("fn recenter_when_sized(")
        .expect("next helper")
        .0;
    assert!(!sizing.contains("64.0"));
    assert!(sizing.contains("overlay.global::<crate::ui::BarSpec>()"));
    assert!(sizing.contains("effective_form("));
    for literal in ["874", "586", "408"] {
        assert!(
            !host.contains(literal),
            "native form widths must come from BarSpec"
        );
    }
    for getter in [
        "get_full_width()",
        "get_compact_width()",
        "get_mini_width()",
        "get_height()",
    ] {
        assert!(sizing.contains(getter), "missing size getter {getter}");
    }
    let bar = read("ui/overlay_bar.slint");
    assert!(bar.contains("preferred-width: root.bootstrap-mode ? 560px : (root.bar-form == 2 ? BarSpec.mini-width : root.bar-form == 1 ? BarSpec.compact-width : BarSpec.full-width);"));
    assert!(bar.contains("preferred-height: BarSpec.height;"));
    assert!(bar.contains("min-width: root.bootstrap-mode ? 440px : (root.bar-form == 2 ? BarSpec.mini-width : root.bar-form == 1 ? BarSpec.compact-width : BarSpec.full-width);"));
    assert!(bar.contains("out property <bool> fits: lay.preferred-width <= self.width;"));
    assert!(bar.contains("in-out property <bool> compact-bar: false;"));
    assert!(bar.contains("in property <int> bar-form: 0;"));
    let fit = bar
        .split_once("out property <bool> all-regions-fit:")
        .expect("fit accessor")
        .1
        .split_once(';')
        .expect("terminated fit binding")
        .0;
    assert!(fit.contains("root.bar-form == 2 || state-capsule.fits"));
    for region in ["state", "voice", "session", "tiles", "window"] {
        assert!(fit.contains(&format!("{region}-capsule.fits")));
        assert!(bar.contains(&format!("{region}-capsule := Capsule {{")));
        let binding = if region == "state" {
            "width: root.bar-form == 2 ? 0px : root.bar-form == 1 ? BarSpec.compact-state : BarSpec.full-state;".to_string()
        } else {
            format!("width: root.bar-form == 2 ? BarSpec.mini-{region} : root.bar-form == 1 ? BarSpec.compact-{region} : BarSpec.full-{region};")
        };
        assert!(
            bar.contains(&binding),
            "capsule {region} uses effective form"
        );
    }
}
