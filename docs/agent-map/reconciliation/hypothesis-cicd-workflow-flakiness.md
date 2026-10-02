# Original CI/CD C09/C11: bounded workflow security and runner flakiness evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_cicd_workflow_flakiness_confirmed.py) inspect frozen GitHub Actions workflow configuration in `.github/workflows/ci.yml`. They do not run CI pipelines, download toolchains, or execute Cargo commands. C09 and C11 remain confirmed mechanisms.

## C09 — Unpinned actions, floating toolchain tags, and absent permissions block

In `.github/workflows/ci.yml`:
- Workflow actions use mutable major version or channel tags (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`) rather than immutable full-length commit SHAs;
- The workflow lacks a top-level `permissions:` block, defaulting to the repository's default token permission scope;
- `dtolnay/rust-toolchain@stable` dynamically pulls the latest stable Rust release on each run, meaning compiler updates can introduce breaking lint or syntax errors without lockstep repository changes.

## C11 — Crate omission in test matrix and unlocked cargo executions

In `.github/workflows/ci.yml`:
- The primary Windows `rust` job [L83-L111](<../../../.github/workflows/ci.yml#L83-L111>) runs formatting, clippy, and unit tests for `overlay-backend`, `slint-experiment`, and `suflyor-tts`, but completely omits shipped crates `suflyor-teratts` and `suflyor-wsola`;
- The `--locked` flag is applied only to the QA-only `ui-mcp` check (`cargo check --locked --bin overlay-host --features ui-mcp`); all primary `cargo test` and `cargo clippy` steps omit `--locked`, allowing floating Cargo dependencies to resolve at build time;
- [L34-L36](<../../../.github/workflows/ci.yml#L34-L36>) configures `concurrency.cancel-in-progress: true`, which automatically terminates in-flight builds when a new push occurs on the same ref, potentially masking intermittent build or test flakes.

## Limits

No GitHub Actions runners were spawned, no remote action tags were resolved, and no CI jobs were cancelled. Original statuses in `candidates.json` remain `confirmed`.
