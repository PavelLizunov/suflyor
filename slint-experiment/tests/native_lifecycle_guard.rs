//! Guard the Windows process-lifecycle boundary.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn assert_create_mutex_isolated_to(path: &Path, allowed: &Path) {
    for entry in fs::read_dir(path).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            assert_create_mutex_isolated_to(&path, allowed);
        } else if path.extension().is_some_and(|extension| extension == "rs") && path != allowed {
            let source = fs::read_to_string(&path).expect("read Rust source");
            assert!(
                !source.contains("CreateMutexW("),
                "process-singleton acquisition outside native adapter: {}",
                path.display()
            );
        }
    }
}

#[test]
fn process_singleton_is_owned_by_the_windows_lifecycle_adapter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let adapter = root.join("src/native/windows/lifecycle.rs");
    let source = fs::read_to_string(&adapter).expect("read lifecycle adapter");
    assert!(source.contains("CreateMutexW("));
    assert!(source.contains("Global\\\\suflyor-overlay-singleton"));
    assert_create_mutex_isolated_to(&root.join("src"), &adapter);

    let wrapper = fs::read_to_string(root.join("src/bin/overlay_host.rs")).expect("read wrapper");
    let windows_host = fs::read_to_string(root.join("src/bin/overlay_host_windows.rs"))
        .expect("read Windows host");
    let bootstrap = fs::read_to_string(root.join("src/bin/overlay_host/app_bootstrap.rs"))
        .expect("read app_bootstrap");
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("read manifest");
    assert!(wrapper.contains("#[cfg(any(windows, target_os = \"macos\"))]"));
    assert!(wrapper.contains("include!(\"overlay_host_windows.rs\")"));
    assert!(wrapper.contains("#[cfg(not(any(windows, target_os = \"macos\")))]"));
    assert!(
        windows_host.contains("native::lifecycle::acquire_singleton")
            || bootstrap.contains("native::lifecycle::acquire_singleton")
    );
    assert!(manifest.contains("autobins = false"));
}

#[test]
fn macos_lifecycle_uses_a_native_file_lock() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/native/macos/lifecycle.rs"))
        .expect("read macOS lifecycle adapter");
    assert!(source.contains("try_lock()"));
    assert!(source.contains("suflyor-overlay-singleton.lock"));
}
