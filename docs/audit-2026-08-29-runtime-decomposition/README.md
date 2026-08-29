# Runtime decomposition evidence — 2026-08-29

## Candidate

- Commit: `9581d70c59c705d8b4e6b532826a45aaf748def2`
- Tree: `557c72c873bfdb24ca9e165570a71b04cb80fb17`
- Branch: `codex/macos-bughunt-tps`
- Scope: dependency-free, behavior-preserving extraction of cohesive functions from four oversized Rust facades.

## Structural acceptance

- `overlay_host_windows.rs`: 5,897 → 4,819 lines.
- `aux_windows.rs`: 3,422 → 83 lines; normal `text_ask`, `help_palette`, `archive`, and `transcript` child modules.
- `local_ai.rs`: 4,332 → 3,141 lines; normal `hardware_profile`, `model_choice`, and `model_state` child modules. Process lifecycle, downloads, integrity verification, extraction, and engine update execution remain in the facade.
- `runtime.rs`: 2,682 → 1,905 lines; normal `summary_plan` and `trigger_detect` child modules. Async orchestration and `RuntimeEvents` dispatch remain in the facade.
- Source-symbol comparison found no missing or added top-level symbol in any split family.
- Brace-aware source comparison found every moved function body unchanged except one Windows-only `ui::Theme` path, qualified as `crate::ui::Theme` to preserve module resolution after extraction.
- No new dependency, textual include, or public child-module API was added.

## Review

- Gemini workers performed four disjoint mechanical extractions; the lead integrated boundaries and visibility.
- Independent correctness review initially found one nested `tile_copy` resolution failure and an altered prompt example. Both were corrected.
- Independent simplicity review findings about broadened visibility and two formatting-only body edits were corrected.
- Both reviewers reported no remaining blocker, high, medium, visibility, or over-engineering finding on recheck.
- The first decomposition candidate `87712b59…` stopped at `cargo fmt --check` on both workers. Native `rustfmt` changed only three Rust files.
- Later native candidates exposed relative child-module paths, crate-root UI imports, one facade visibility, and static guards that scanned moved definitions in their former files. The module seams were repaired, and the guards now follow the facade into the owning child modules.
- The last Windows Clippy pass exposed one Windows-only `ui::Theme` path that macOS does not compile; qualifying that path produced the final candidate above.

## Exact-SHA remote evidence map

### Windows

- Worker alias: `windows-worker`
- Scheduled task: `DshSuflyorGate9581d70c`
- Remote evidence alias: `windows-gate-9581d70c-decomposition`
- Expected markers: `manifest.txt`, `gate.log`, `exit.txt`, `finished-utc.txt`, `pid.txt`
- Command: `scripts/git-gate-native.ps1 manual -Full`
- Result: passed (`exit 0`, 2026-08-29T18:38:57.1882572Z)

### macOS

- Worker alias: `mac-worker`
- Remote evidence alias: `macos-gate-9581d70c-decomposition`
- Expected markers: `manifest.txt`, `gate.log`, `exit.txt`, `finished-utc.txt`, `pid.txt`
- Command: `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 ./scripts/git-gate-macos.sh`
- Result: passed (`exit 0`, 2026-08-29T18:10:41Z)

## Exact-SHA live UI/MCP smoke

- Remote evidence alias: `windows-ui-mcp-9581d70c-decomposition-r2`.
- Exact QA build: `CARGO_INCREMENTAL=0 SLINT_EMIT_DEBUG_INFO=1 cargo build --locked --bin overlay-host --features ui-mcp --manifest-path slint-experiment/Cargo.toml`; passed (`exit 0`, 2026-08-29T18:44:39.9309224Z).
- The feature-enabled binary ran in the interactive Windows console with the Slint software renderer and MCP on port 9123. Its element tree was populated (122 bar elements; 200 Settings/Diagnostics elements).
- MCP opened the 1280×64 bar, 640×680 Help, 520×420 Palette, 720×540 Archive, and 720×600 Settings/Diagnostics surfaces. Archive was recorded without a screenshot to avoid exposing session data.
- The bar screenshot received a pixel-grounded review with no clipping, overlap, stretching, tofu, unreadable text, or broken gaps. Help, Palette, and Diagnostics screenshots were captured, but the harness omitted their image payload during review; no visual-pass claim is made for those three captures.
- Settings/Diagnostics contained the exact registered list for all 13 shortcuts. A disposable Notepad selection was used for `Shift+Alt+1`; capture overlays were canceled with `Esc`. Registration and distinct dispatch checks passed for `F1`, `F3`, `F4`, `F6`, `F7`, `F8`, `Shift+F8`, `Ctrl+F8`, `F9`, `Shift+F9`, `Shift+Alt+1`, `Shift+Alt+2`, and `Shift+Alt+3`.
- External AI/Vision completion and F8 route-persistence behavior were not exercised; this refactor changed no routing, visible text, or Slint source.
- The normal non-MCP exact-candidate binary was rebuilt (`exit 0`, 2026-08-29T19:00:31.2894732Z) and left running in the Windows console; MCP port 9123 was not listening.
