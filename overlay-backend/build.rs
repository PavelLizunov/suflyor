//! Build script: compiles the macOS-only Objective-C audio bridges
//! (`native/macos/mic_capture.m` and `native/macos/system_capture.m`) into
//! static libs. No-op on every other target — Windows keeps its WASAPI path
//! and other platforms use the unsupported audio seam.

fn main() {
    println!("cargo:rerun-if-changed=native/macos/mic_capture.m");
    println!("cargo:rerun-if-changed=native/macos/system_capture.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        build_mic_capture();
        build_system_capture();
    }
}

#[cfg(target_os = "macos")]
fn build_mic_capture() {
    cc::Build::new()
        .file("native/macos/mic_capture.m")
        .flag("-fblocks")
        .compile("mic_capture");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=AVFoundation");
    println!("cargo:rustc-link-lib=framework=AVFAudio");
}

#[cfg(target_os = "macos")]
fn build_system_capture() {
    cc::Build::new()
        .file("native/macos/system_capture.m")
        .flag("-fobjc-arc")
        .flag("-fblocks")
        .compile("system_capture");
    println!("cargo:rustc-link-lib=framework=CoreAudio");
}

#[cfg(not(target_os = "macos"))]
fn build_mic_capture() {
    // The `cc` build-dependency is macOS-target-gated, so the bridge can
    // only be compiled on a macOS host. Cross-compiling overlay-backend to
    // macOS from another host is not supported.
    eprintln!("overlay-backend: macOS targets must be built on macOS (mic_capture bridge)");
    std::process::exit(1);
}

#[cfg(not(target_os = "macos"))]
fn build_system_capture() {
    eprintln!("overlay-backend: macOS targets must be built on macOS (system_capture bridge)");
    std::process::exit(1);
}
