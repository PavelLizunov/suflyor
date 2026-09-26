# O4: Slint Reactivity vs. Rust Concurrency

## 1. UI Thread Ownership vs. Async Background Tasks
- **Slint Thread Affinity**: Slint's event loop and window handles (`slint::ComponentHandle`, `slint::Window`) are strictly single-threaded and `!Send`. Any mutation of UI properties or model state must occur on the thread that runs the Slint event loop.
- **Cross-Thread Bridge via Weak Handles**:
  - Background asynchronous tasks (Tokio threads, std threads) capture a `slint::Weak<T>` reference.
  - To push updates back to the UI, the worker upgrades the weak handle and schedules execution via `slint::invoke_from_event_loop(move || { ... })`.
  - If the window was closed or dropped, `weak.upgrade()` returns `None`, safely terminating the callback without panic.

## 2. The Reused Settings Window Hazard
- **Singleton Lifecycle**: To ensure fast responsive opening when clicking the Settings gear or pressing hotkeys, Suflyor instantiates the Settings window once and reuses it across the process lifetime (`.show()` on open, `.hide()` on dismiss).
- **Stale Transient Status**:
  - In a freshly created window, transient feedback properties default to empty or idle.
  - In a reused window, strings like `*_status`, `*_result`, and flags like `component_busy` persist indefinitely from prior operations unless explicitly cleared.
- **The `populate_token_status` Invariant**:
  - Every time Settings is shown, `populate_token_status` runs synchronously before the window is revealed.
  - It resets every transient status string to an empty state, reads fresh configuration from disk, and launches async validation probes afresh.
  - The static integration test `settings_reset_guard` verifies that all transient UI property names defined in `.slint` have corresponding reset statements in Rust.

## 3. Optimistic State-Flips vs. Confirmed Commit Branches
- **Anti-Pattern**: Flipping UI toggle switches or updating stored configuration *before* an asynchronous external operation (e.g. verifying an API token, downloading a model weight, testing an endpoint) has returned success.
- **Enforced Invariant**:
  - The UI enters a pending or loading state (`*_checking = true`).
  - Configuration files are only updated on the confirmed-success branch.
  - If an operation fails, the UI reverts to the prior verified state and renders a localized error message, preventing the interface from displaying false success indicators.
