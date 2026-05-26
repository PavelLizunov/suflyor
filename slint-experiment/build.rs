// Compiles ui/replay.slint into generated Rust pulled in via
// `slint::include_modules!()` in main.rs. Re-run whenever the .slint
// file changes (slint-build emits the appropriate rerun-if-changed
// directive).
fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint_build::compile("ui/replay.slint")?;
    Ok(())
}
