//! Regression guards for the RC3 installer and read-aloud hotkey fixes.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

#[test]
fn installer_stops_only_the_exact_installed_copy_before_overwrite() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let nsi =
        fs::read_to_string(root.join("../scripts/slint-installer.nsi")).expect("read installer");
    let guard = fs::read_to_string(root.join("../scripts/stop-installed-suflyor.ps1"))
        .expect("read process guard");
    let guard_pos = nsi.find("-CheckOnly").expect("installer check");
    let file_pos = nsi
        .find("File \"..\\slint-experiment\\target\\release\\${PRODUCT_EXE}\"")
        .expect("host file");
    assert!(
        guard_pos < file_pos,
        "process guard must run before overwrite"
    );
    assert!(guard.contains("[IO.Path]::GetFullPath($process.Path)"));
    assert!(guard.contains("$targets.ContainsKey($path)"));
    assert!(!guard.to_ascii_lowercase().contains("taskkill"));
    assert!(!guard.contains("Stop-Process -Name"));
}

#[test]
fn read_aloud_hotkeys_wait_for_release_and_ignore_phantom_pointer_up() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let host = fs::read_to_string(root.join("src/bin/overlay_host_windows.rs"))
        .expect("read Windows host");
    let win32 = fs::read_to_string(root.join("src/win32.rs")).expect("read win32");
    let capture =
        fs::read_to_string(root.join("ui/capture_overlay.slint")).expect("read capture UI");
    assert!(host.contains("after_read_aloud_hotkey_release(40, copy_selection)"));
    assert!(win32.contains("GetAsyncKeyState"));
    assert!(win32.contains("!down(VK_SHIFT) && !down(VK_MENU) && !down(VK_CONTROL)"));
    let stray_guard = capture.find("if (!root.dragging)").expect("stray-up guard");
    let selection = capture
        .find("root.region-selected(")
        .expect("region callback");
    assert!(stray_guard < selection);
    assert!(capture.contains("root.rw >= 16px && root.rh >= 16px"));
}
