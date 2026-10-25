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

3. Drive the server with `scripts/slint_mcp.py` (standard-library Python; run
   `python scripts/slint_mcp.py --help`). It does the JSON-RPC handshake
   (`initialize` → `notifications/initialized` → `tools/call`) against
   `http://127.0.0.1:9123/mcp`, or against `--url` / `SLINT_MCP_URL` when the
   app runs on another machine behind a forwarded port.
4. `slint_mcp.py windows` lists the windows; `shot <window> out.png` captures
   one (`bar`, `settings`, `palette`, `archive`, `transcript`, `help`, or a
   handle). To judge a single control, use `shot-element <window> <query>
   out.png`; `shot-elements <window> <dir>` writes one PNG per labelled or
   interactive element plus `index.tsv` with role, label, value and box, so
   every element of a window can be reviewed one by one.
5. Open surfaces and interact through Slint MCP when element handles exist. The accessibility tree may contain only the root; this is a known limitation, not a passed audit.
6. When Settings cannot be opened through the tree, take a bar screenshot, combine its gear position with the returned window position, and use the existing `scripts/sim_click.ps1`. Never guess coordinates without a fresh screenshot and window properties.
7. Inspect the screenshots visually. Scroll long pages and capture the continuation.
8. If the same geometry defect survives two edit/build/MCP cycles, stop tuning
   literal sizes. Capture the overflowing child and parent layout geometry, then
   ask a read-only Qwen or independent agent for a fresh review before editing
   again. In Slint layouts, `height`/`min-height` can change rendered geometry
   without increasing a parent ScrollView's extent; verify `preferred-height`.

## Linux / Hyprland QA

Keep the Windows procedure above for Windows. On the Omarchy worker, build the
same candidate with Slint debug metadata and the QA feature:

```bash
CARGO_INCREMENTAL=0 SLINT_EMIT_DEBUG_INFO=1 cargo build --locked --no-default-features --bin overlay-host --features ui-mcp --manifest-path slint-experiment/Cargo.toml
SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=9123 slint-experiment/target/debug/overlay-host
```

Use `python3 scripts/slint_mcp.py` for the same window, element, click and
screenshot commands. `overlay-host.exe`, HWND coordinates and `sim_click.ps1`
are Windows-only. Use `hyprctl -j clients` and `hyprctl -j monitors` for native
window geometry. For actual composed frames and z-order, use a compositor or VM
framebuffer dump (for example, `qm monitor screendump` on the VM host). `grim`
draws `no_screen_share` windows black, even when another window covers them; it
cannot establish their visible content or z-order. Keep MCP captures for Slint
content and colour comparisons.

## Required coverage

- Before the first edit, create `docs/audit-YYYY-MM-DD-<task>/` and capture the
  baseline. Keep each before/after pair at the same window size, DPI, theme,
  language, query/data state and scroll position. Record those conditions in
  the directory's `README.md`; an after-only image is not evidence. When the
  task closes, remove the folder from the tree and index it in
  `docs/HISTORY-INDEX.md` (`docs/AGENTS.md` section 6).
- For a local component change, capture every affected state before and after
  the change. For list/search surfaces use the smallest state matrix that can
  expose the defect: empty, one/few rows, enough rows to show the scrollbar,
  no-match/long text, selected/busy where applicable, and declared minimum
  window size.
- For a shared Settings primitive or layout change, visit every Settings sidebar entry (currently 17) at 720×600.
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
repeat it on every Settings tab: hotkeys are process-global, so that would
repeat the same actions without increasing coverage.

- Windows: exercise all 13 registered shortcuts through Windows input:
  `F1`, `F3`, `F4`, `F6`, `F7`, `F8`, `Shift+F8`, `Ctrl+F8`, `F9`,
  `Shift+F9`, `Shift+Alt+1`, `Shift+Alt+2`, `Shift+Alt+3`.
- Wayland: dispatch those same 13 shortcuts through Hyprland, plus `Super+F9`
  microphone PTT press/release, `Super+Shift+F9` system-audio PTT press/release,
  and `Super+F10` show. `WAYLAND_BINDINGS` in `hotkeys.rs` has 18 entries,
  including the two release bindings. Check both PTT edges, not only key-down.
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
- the selected repository gate passed;
- the summary names the windows/tabs checked and any unverified state.

Leave the normal non-MCP release binary running for the user's final visual acceptance. RC prereleases follow the standing authorization in `AGENTS.md`; a stable release still requires the user's explicit `релизь`.

## Three checks for every UI diff

Every UI diff passes these three checks before the user is shown. Present the results as evidence, not as a verdict.

- **(a) Screenshots and a written UI checklist.** Capture the key windows in
  the relevant states (Settings, bar, tile) as "Run the audit" describes, and
  write the checklist: every string is in `@tr` and in the `.po`; no emoji
  where an SVG belongs; button states (enabled, disabled, active marker) are
  logical; status text matches the real state (no "Готово" when not done);
  signs, punctuation and spacing.
- **(b) UI-review agent** on the `.slint` and wiring diff. This is the category
  that slips through static gates.
- **(c) Slint MCP** on the `ui-mcp` QA build from "Run the audit", not on the
  shipped release binary. Where element handles exist, read real control text
  and state through MCP instead of trusting pixels alone; step 5 above covers
  the root-only tree.

## Recurring UI bug shapes

Check the `.rs` side, not just `.slint`:

1. **Stale status on a REUSED window** — `populate_token_status` clears or
   reseeds idle transient `*_status`/`*_result` strings on reopen. It preserves
   active local-model download/switch and Strata Vision job status until the
   worker finishes. Do not erase live progress or Cancel state. (Idle stale
   status caused the user's lingering "Готово: умная модель (12B)".)
2. **Optimistic state-flip before an async result** — writing config + UI to
   the new value *before* the operation confirms; on failure the UI lies.
   Commit only on the confirmed-success branch.
3. **A `.slint` default property with NO Rust setter** — renders fake data
   forever (palette `recent-chips: ["kubernetes",…]` had no `set_recent_chips`
   → always shown). Grep for `set_<prop>`; if absent, the default IS the
   production value.
4. **emoji where the chrome standard is SVG** / **@tr↔.po drift after a string edit**.
