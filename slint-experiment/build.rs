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
    let config =
        slint_build::CompilerConfiguration::new().with_bundled_translations("translations");
    slint_build::compile_with_config("ui/index.slint", config)?;
    Ok(())
}
