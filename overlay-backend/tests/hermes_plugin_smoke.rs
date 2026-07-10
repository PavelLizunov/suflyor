//! Live smoke for the in-app Hermes-plugin installer (run on demand):
//!
//! ```text
//! HERMES_HOME=<scratch dir with a COPY of a real config.yaml/.env> \
//!   cargo test --test hermes_plugin_smoke -- --ignored
//! ```
//!
//! Exercises the fs glue the unit tests can't (dir creation, file writes,
//! read-modify-write) against real-world file shapes without touching the
//! real Hermes home. Ignored by default so the gate never depends on env.
//! NB: the file name deliberately avoids the word "install" — Windows UAC
//! Installer Detection demands elevation for unmanifested *install*.exe.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[test]
#[ignore]
fn install_into_scratch_hermes_home() {
    let home = std::env::var("HERMES_HOME").expect("set HERMES_HOME to a scratch dir");
    let status =
        overlay_backend::hermes_install::install_plugin("http://127.0.0.1:8654", "smoke-token-123")
            .expect("install_plugin failed");
    println!("status: {status}");

    let root = std::path::Path::new(&home);
    let yaml = std::fs::read_to_string(root.join("plugins/suflyor/plugin.yaml"))
        .expect("plugin.yaml written");
    assert!(yaml.contains("name: suflyor"));
    let init = std::fs::read_to_string(root.join("plugins/suflyor/__init__.py"))
        .expect("__init__.py written");
    assert!(init.contains("def register(ctx)"));
    let env = std::fs::read_to_string(root.join(".env")).expect(".env written");
    assert!(env.contains("SUFLYOR_BRIDGE_TOKEN=smoke-token-123"));
    let cfg = std::fs::read_to_string(root.join("config.yaml")).expect("config.yaml written");
    assert!(cfg.contains("- suflyor"));

    // Second run must be idempotent and report the already-enabled path.
    let again =
        overlay_backend::hermes_install::install_plugin("http://127.0.0.1:8654", "smoke-token-123")
            .expect("re-install failed");
    println!("again: {again}");
    assert!(again.contains("уже включён"));

    // «Взять ключ из локального Hermes»: first run creates api_server+key,
    // second run reads the SAME key back without touching the file.
    let (key1, changed1) =
        overlay_backend::hermes_install::ensure_api_server().expect("api setup failed");
    println!("api key created: changed={changed1}");
    assert!(changed1);
    assert!(!key1.is_empty());
    let (key2, changed2) =
        overlay_backend::hermes_install::ensure_api_server().expect("api re-read failed");
    assert!(!changed2);
    assert_eq!(key1, key2);
    let cfg2 = std::fs::read_to_string(root.join("config.yaml")).expect("config re-read");
    assert!(cfg2.contains("api_server:"));
    assert!(cfg2.contains("enabled: true"));
}
