# Original CI/CD C01/C02/C03: bounded native gate classification bypass evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_cicd_native_gate_classification_confirmed.py) inspect frozen native git gate classification logic in `scripts/git-gate-native.ps1` and compiled knowledge base assets in `overlay-backend/src/kb.rs`. They do not run PowerShell gate scripts or trigger Cargo builds. C01, C02, and C03 remain confirmed mechanisms.

## C01 — Knowledge base markdown classified as docs-only despite compilation into binary

In `scripts/git-gate-native.ps1` [L74-L81](<../../../scripts/git-gate-native.ps1#L74-L81>), `$docsOnly` checks if all changed files match the extension regex `'(^|/)[^/]+\.(md|html|txt)$'`. If so, `$tier = 'docs'` and the script exits with code 0 at [L132-L135](<../../../scripts/git-gate-native.ps1#L132-L135>) without running Cargo compilation, Clippy, or unit tests.
However, files under `overlay-backend/knowledge/` (`glossary.md`, `commands.md`, `patterns.md`) are embedded directly into Rust binaries via `include_str!` in `overlay-backend/src/kb.rs` [L20-L22](<../../../overlay-backend/src/kb.rs#L20-L22>), meaning changes to these markdown files affect binary code and runtime behavior.

## C02 — Crate membership omissions

In `scripts/git-gate-native.ps1` [L21-L27](<../../../scripts/git-gate-native.ps1#L21-L27>), `$crateOrder` lists only five crates:
`overlay-backend`, `slint-experiment`, `suflyor-wsola`, `suflyor-tts`, and `suflyor-teratts`.
Standalone modules and platform sidecars such as `suflyor-mlx` (Swift/macOS) are omitted from crate change detection.
Furthermore, when not in the `commit` stage [L37-L43](<../../../scripts/git-gate-native.ps1#L37-L43>), `$diffArguments` defaults to empty, comparing against the working tree rather than staged changes.

## C03 — Slint UI-only diff skips Clippy and runs hardcoded guard test list

In `scripts/git-gate-native.ps1` [L151-L172](<../../../scripts/git-gate-native.ps1#L151-L172>), when a diff under `slint-experiment/` touches only UI markup without `.rs`, `.toml`, or `.lock` modifications (`$slintUiOnly` is true):
- `cargo check --locked ... --bin overlay-host` runs;
- a hardcoded array of 13 integration tests (`$guards`) is executed via `--test <name>`;
- `cargo clippy` is completely skipped, and any new tests outside the hardcoded 13 guards are not executed.

## Limits

No Git hooks or PowerShell script runners were executed on host platforms, and no actual bypass was triggered against production CI. Original statuses in `candidates.json` remain `confirmed`.
