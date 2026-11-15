# Suflyor build-helper patch

Source: the published crates.io spin_on 0.1.1 archive.
SHA-256: `076e103ed41b9864aa838287efe5f4e3a7a0362dd00671ae62a212e5e4612da2`.
The upstream Apache-2.0 OR MIT licenses and attribution are retained.

Slint 1.17.1 uses this no_std helper at build time. The published helper's only
dependency is pin-utils, archived in RUSTSEC-2026-0441. Both the published 0.1.1
release and the checked 1.18.1 Slint release still use it.

Local changes: use stable core::pin::pin! (Rust 1.68+) instead of pin_mut!,
remove the external dependency, update dependency-count documentation, and test
a !Unpin future across multiple pending polls. The public API, polling loop and
waker implementation are unchanged. The upstream ready/count tests are retained.

This is a host-crate Cargo patch, not a security advisory exception. Remove it
when Slint uses a maintained helper without pin-utils; regenerate Cargo.lock and
rerun the targeted native and supply-chain checks then.
