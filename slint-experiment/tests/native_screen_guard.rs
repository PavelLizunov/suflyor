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
