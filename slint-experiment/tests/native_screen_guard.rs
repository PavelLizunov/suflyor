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
    assert!(screen_rs.contains("suflyor_macos_copy_active_displays"));
    assert!(screen_rs.contains("pub fn display_union"));
    assert!(screen_rs.contains("pub fn cursor_position"));
    assert!(screen_rs.contains("recognize_text_from_bgra"));
    assert!(screen_rs.contains("checked_mul(height as usize)"));
    assert!(screen_rs.contains("bgra.len() != expected"));

    let screen_m =
        fs::read_to_string(root.join("src/native/macos/screen.m")).expect("read screen.m");
    assert!(screen_m.contains("SCShareableContent"));
    assert!(screen_m.contains("SCScreenshotManager"));
    assert!(screen_m.contains("CGGetActiveDisplayList"));
    assert!(screen_m.contains("CGEventGetLocation"));
    assert!(screen_m.contains("excludingApplications:@[self_application]"));
    assert!(screen_m.contains("application.processID == self_pid"));
    assert!(screen_m.contains("excludingWindows:own_windows"));
    assert!(screen_m.contains("self_application == nil && own_windows.count == 0"));
    assert!(screen_m.contains("VNRecognizeTextRequest"));
    assert!(screen_m.contains("@autoreleasepool"));
    assert!(screen_m.contains("request.automaticallyDetectsLanguage = YES"));
    assert!(screen_m.contains("if (!*out_text)"));
}

#[test]
fn canonical_ocr_route_uses_the_platform_local_engine_off_thread() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let capture = fs::read_to_string(root.join("src/bin/overlay_host/vision_capture.rs"))
        .expect("read vision capture");
    let host = fs::read_to_string(root.join("src/bin/overlay_host_windows.rs"))
        .expect("read canonical host");

    assert!(capture.contains("fn local_ocr_available()"));
    assert!(capture.contains("overlay_backend::ocr::is_available()"));
    assert!(capture.contains("slint_replay::native::screen::recognize_text_from_bgra"));
    assert!(capture.contains("tokio::task::spawn_blocking(move || run_local_ocr"));
    assert!(capture.contains("super::fill_ocr_error_tile(weak_ocr, ui_is_ru)"));
    assert!(host.contains("pub(crate) fn fill_ocr_error_tile"));
    assert!(host.contains("Text recognition failed."));
    assert!(host.contains("OCR · Tesseract"));
    assert!(host.contains("OCR · Apple Vision"));
    assert!(host.contains("*(no text recognized)*"));
}
