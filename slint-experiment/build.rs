fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dev_debug_info = std::env::var("PROFILE").is_ok_and(|profile| profile != "release")
        || std::env::var("SLINT_EMIT_DEBUG_INFO").is_ok_and(|value| value == "1");
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("translations")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None)
        .with_debug_info(dev_debug_info);
    slint_build::compile_with_config("ui/index.slint", config)?;

    #[cfg(target_os = "macos")]
    {
        cc::Build::new()
            .file("src/native/macos/window.m")
            .file("src/native/macos/status.m")
            .file("src/native/macos/clipboard.m")
            .file("src/native/macos/screen.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("suflyor_appkit");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
        println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=Vision");
        println!("cargo:rerun-if-changed=src/native/macos/window.m");
        println!("cargo:rerun-if-changed=src/native/macos/status.m");
        println!("cargo:rerun-if-changed=src/native/macos/clipboard.m");
        println!("cargo:rerun-if-changed=src/native/macos/screen.m");
    }

    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=app icon embed skipped ({e})");
        }
    }

    Ok(())
}
