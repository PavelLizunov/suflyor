//! Guard the Windows GDI screenshot boundary.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn assert_get_dibits_isolated_to(path: &Path, allowed: &Path) {
    for entry in fs::read_dir(path).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            assert_get_dibits_isolated_to(&path, allowed);
        } else if path.extension().is_some_and(|extension| extension == "rs") && path != allowed {
            let source = fs::read_to_string(&path).expect("read Rust source");
            assert!(
                !source.contains("GetDIBits("),
                "GDI screenshot acquisition outside native adapter: {}",
                path.display()
            );
        }
    }
}

#[test]
fn gdi_screenshot_acquisition_is_owned_by_the_windows_adapter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let adapter = root.join("src/native/windows/screen.rs");
    let source = fs::read_to_string(&adapter).expect("read screen adapter");
    assert!(source.contains("GetDIBits("));
    assert!(source.contains("biHeight: -h"));
    assert_get_dibits_isolated_to(&root.join("src"), &adapter);
}

#[test]
fn macos_screencapturekit_adapter_is_wired() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let build_rs = fs::read_to_string(root.join("build.rs")).expect("read build.rs");
    assert!(build_rs.contains("src/native/macos/screen.m"));
    assert!(build_rs.contains("framework=ScreenCaptureKit"));
    assert!(build_rs.contains("framework=Vision"));

    let mod_rs = fs::read_to_string(root.join("src/native/mod.rs")).expect("read mod.rs");
    assert!(mod_rs.contains("path = \"macos/screen.rs\""));

    let screen_rs =
        fs::read_to_string(root.join("src/native/macos/screen.rs")).expect("read screen.rs");
    assert!(screen_rs.contains("suflyor_macos_capture_display_bgra"));
    assert!(screen_rs.contains("recognize_text_from_bgra"));

    let screen_m =
        fs::read_to_string(root.join("src/native/macos/screen.m")).expect("read screen.m");
    assert!(screen_m.contains("SCShareableContent"));
    assert!(screen_m.contains("SCScreenshotManager"));
    assert!(screen_m.contains("VNRecognizeTextRequest"));
}
