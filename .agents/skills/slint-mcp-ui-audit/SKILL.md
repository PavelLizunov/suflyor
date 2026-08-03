---
name: slint-mcp-ui-audit
description: Validate Suflyor UI changes in the running Slint application through its embedded Slint MCP server. Use after any .slint edit or Rust change that affects visible UI, layout, text, enabled states, Settings tabs, overlay windows, or release presentation; use before declaring a UI task done, committing it, or handing a build to the user.
---

# Slint MCP UI Audit

Treat live visual verification as mandatory for UI changes. Compilation and tests do not prove that a Slint layout is usable.

## Run the audit

1. Build the exact QA binary being validated. The embedded Slint MCP server is
   compiled in **only** by the `ui-mcp` Cargo feature; setting
   `SLINT_MCP_PORT` / `SLINT_EMIT_DEBUG_INFO` on a normal build does not
   enable it. Debug info must be present **while Slint compiles the UI**;
   setting it only when the finished binary starts leaves MCP with a root-only
   element tree. Build the feature-enabled binary with exactly:

   ```powershell
   $env:CARGO_INCREMENTAL='0'
   $env:SLINT_EMIT_DEBUG_INFO='1'
   cargo build --locked --bin overlay-host --features ui-mcp --manifest-path slint-experiment/Cargo.toml
   ```

   Audit that binary (`slint-experiment/target/debug/overlay-host.exe`). A
   binary built without `--features ui-mcp` has no MCP server and cannot pass
   this audit. Kill existing `overlay-host.exe` and `suflyor-tts.exe`
   instances before a release build.
2. Launch that same binary with:

   ```powershell
   $env:SLINT_EMIT_DEBUG_INFO='1'
   $env:SLINT_MCP_PORT='9123'
   ```

3. Connect to `http://127.0.0.1:9123/mcp` using JSON-RPC:
   `initialize` → `notifications/initialized` → `tools/call`.
4. Call `list_windows`, identify windows by `get_window_properties`, then use `take_screenshot`.
5. Open surfaces and interact through Slint MCP when element handles exist. The accessibility tree may contain only the root; this is a known limitation, not a passed audit.
6. When Settings cannot be opened through the tree, take a bar screenshot, combine its gear position with the returned window position, and use the existing `scripts/sim_click.ps1`. Never guess coordinates without a fresh screenshot and window properties.
7. Inspect the screenshots visually. Scroll long pages and capture the continuation.
8. If the same geometry defect survives two edit/build/MCP cycles, stop tuning
   literal sizes. Capture the overflowing child and parent layout geometry, then
   ask a read-only Qwen or independent agent for a fresh review before editing
   again. In Slint layouts, `height`/`min-height` can change rendered geometry
   without increasing a parent ScrollView's extent; verify `preferred-height`.

## Required coverage

- Before the first edit, create `docs/audit-YYYY-MM-DD-<task>/` and capture the
  baseline. Keep each before/after pair at the same window size, DPI, theme,
  language, query/data state and scroll position. Record those conditions in
  the directory's `README.md`; an after-only image is not evidence.
- For a local component change, capture every affected state before and after
  the change. For list/search surfaces use the smallest state matrix that can
  expose the defect: empty, one/few rows, enough rows to show the scrollbar,
  no-match/long text, selected/busy where applicable, and declared minimum
  window size.
- For a shared Settings primitive or layout change, visit all 16 Settings tabs at 720×600.
- Check clipping, overflow, accidental stretching, large gaps, scroll reachability, button enabled/selected state, translations, and status accuracy.
- Treat geometry as pass/fail, not taste: header icon/grip/text centres differ
  by at most 2 px; fixed-row top/bottom gaps differ by at most 2 px; controls
  keep at least 8 px from a row border; overlay scrollbars get a constant 14 px
  content gutter; interactive targets are at least 24×24 px.
- Run changed user-facing states once in English and Russian. Rust-built text
  is not covered by the `.slint` i18n guard, so inspect MCP element values as
  well as pixels.
- Never commit screenshots containing keys, URLs/IPs, user paths, real session
  titles or transcript text. Use a no-match query or synthetic data; keep any
  unavoidable private runtime evidence outside the repository.
- For transparent-window colours, trust Slint MCP screenshots. Computer-use screenshots are not colour ground truth for this project; use `CopyFromScreen` only as the documented fallback.

## Global-hotkey smoke

Run this once against the exact MCP-audited binary, after the page pass. Do not
repeat it on every Settings tab: hotkeys are process-global, so that would add
208 duplicate actions without increasing coverage.

- Exercise every registered shortcut through Windows input:
  `F1`, `F3`, `F4`, `F6`, `F7`, `F8`, `Shift+F8`, `Ctrl+F8`, `F9`,
  `Shift+F9`, `Shift+Alt+1`, `Shift+Alt+2`, `Shift+Alt+3`.
- Confirm both registration in Settings > Diagnostics and the distinct dispatch
  log/result for every shortcut. Registration alone is not a functional pass.
- Cancel region-capture surfaces with `Esc` before continuing. Use disposable
  selected text for `Shift+Alt+1` because that path intentionally touches the
  clipboard.
- For `F8`, also verify that changing the Vision route persists after Settings
  closes and that the resulting capture path matches the saved route. Record any
  route that cannot be exercised because its external service is unavailable.

## Completion gate

Do not say a UI change is done until:

- the affected screenshots were inspected;
- any shared Settings change received the full tab pass;
- the global-hotkey smoke passed, or every unverified shortcut is named;
- the normal repository gate passed;
- the summary names the windows/tabs checked and any unverified state.

Leave the normal non-MCP release binary running for the user's final visual acceptance. Never publish a release without the user's explicit `релизь`.
