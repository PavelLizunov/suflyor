# O8: Whole-Map Adversarial Review & Contradiction Audit

## 1. Documentation vs. Real Implementation Contradictions
- **The "24 Domain Modules" Claim**:
  - *Claim*: Historical handoffs and older docs state that `overlay-backend` exports exactly 24 modules.
  - *Reality*: Direct inspection of `overlay-backend/src/lib.rs` reveals a significantly larger module surface (>35 modules), including platform-gated native modules (`audio_macos`, `mlx_runtime`, `audio_unavailable`).
- **Phase M4 Vector Memory Status**:
  - *Claim*: Memory architecture docs suggest hybrid RRF vector search with a `llama-server` sidecar.
  - *Reality*: Codebase inspection confirms `memory_items.embedding_status` is hardcoded to `'none'`. No vector table, cosine similarity, or embedding sidecar is currently linked in production; memory search is pure keyword-based prefix matching.
- **WSOLA Boundary**:
  - *Claim*: Older tickets referred to WSOLA as a helper process.
  - *Reality*: `suflyor-wsola` is an in-process Rust static library dependency invoked directly from `playback.rs` and sidecars, not an IPC subprocess.

## 2. Clippy & Lints Denial Enforcement
- Both `overlay-backend` and `slint-experiment` enforce strict zero-panic / zero-unwrap policies outside `#[cfg(test)]`:
  - `clippy::unwrap_used = "deny"`
  - `clippy::expect_used = "deny"`
  - `clippy::panic = "deny"`
- Production code consistently handles errors via `Result`, `anyhow::Context`, or fallback default branches.

## 3. Concurrency & Reused Window Hazards
- **Settings Window Reuse**: Because Slint does not destroy and recreate the Settings window on close, all transient status messages (`*-status`, `*-result`) linger across sessions unless explicitly wiped by `populate_token_status`.
- **Async Model Dropdown Race**: In `settings_ai.rs:452-503`, `fetch_models` lacks a generation counter. Rapid endpoint URL switching could allow an older slow network response to overwrite a newer model list.

## 4. Platform Parity Realities
- **Stealth Mode on macOS**: Windows fully supports hardware-accelerated screen capture exclusion via `WDA_EXCLUDEFROMCAPTURE`. On macOS, `set_stealth` explicitly returns `Err(Unsupported)` because AppKit lacks a direct global capture affinity API equivalent to Windows DWM. Instead, macOS implements self-exclusion via ScreenCaptureKit filter predicates during internal screen grabs.
