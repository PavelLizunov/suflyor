//! Tauri build-script: regenerates capability bindings + tauri.conf.json
//! checks on every `cargo build`. Required by tauri-build per Tauri 2 docs.

fn main() {
    tauri_build::build()
}
