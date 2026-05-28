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
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("translations")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/index.slint", config)?;
    Ok(())
}
