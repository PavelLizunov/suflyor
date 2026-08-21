// Compiles ui/index.slint which transitively imports the per-window
// .slint files and re-exports their components. slint_build's
// compile() only emits one output, so multiple top-level compile()
// calls would clobber each other; the single-root pattern is the
// standard way to expose multiple components from one crate.
//
// Phase D1 — bundled translations from translations/<lang>/LC_MESSAGES/
// *.po files. With this enabled, `@tr("msgid")` in .slint files returns
// the translated string for the language selected at runtime via
// `slint::select_bundled_translation("ru")`. Default is English (msgid).
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Phase E6 v38 — THE Russian-i18n fix. `with_bundled_translations`
    // alone silently did nothing user-visible because Slint's DEFAULT
    // translation context is the *component name* (each `@tr("x")` inside
    // `OverlayBarWindow` compiles to a lookup keyed by msgctxt=
    // "OverlayBarWindow"). Our hand-written .po has NO msgctxt, so every
    // lookup missed → the UI stayed English even though "ru" was selected
    // (`select_bundled_translation` returns Ok against an effectively
    // empty table). `DefaultTranslationContext::None` makes `@tr` look up
    // by bare msgid, matching the context-free .po. (If we ever switch to
    // slint-tr-extractor it must be run with --no-default-translation-
    // context to stay consistent.)
    // Headless interaction tests use Slint's official ElementHandle API. Keep
    // element metadata in dev/test builds, but do not carry it into the release
    // installer unless SLINT_EMIT_DEBUG_INFO explicitly requests it (MCP QA).
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

    // Embed the app icon into the .exe so Explorer / the taskbar / a
    // pinned shortcut show the suflyor mark instead of the generic
    // Windows default. Best-effort: if the Windows SDK resource compiler
    // (rc.exe) is missing, log a warning and continue — the NSIS
    // installer also points the Start-menu/Desktop shortcuts at the
    // same icon.ico, so the user-facing launchers stay branded either way.
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
