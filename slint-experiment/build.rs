// Compiles ui/index.slint which transitively imports replay.slint and
// overlay_spike.slint and re-exports their components. slint_build's
// compile() only emits one output, so multiple top-level compile()
// calls would clobber each other; the single-root pattern is the
// standard way to expose multiple components from one crate.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint_build::compile("ui/index.slint")?;
    Ok(())
}
