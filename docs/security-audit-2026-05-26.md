# Security audit snapshot — 2026-05-26 (v0.0.15)

> ⚠️ **HISTORICAL — pre-Slint stack.** This snapshot audited the original
> **Tauri 2 + React 19 + Vite** stack as it stood on 2026-05-26. Two days later
> the entire UI was rewritten in **Rust + Slint** (Phase 7 cut, 2026-05-28):
> there is no longer any Tauri, React, Vite, WebView2, npm, or GTK dependency,
> so the NPM-audit / capability-split / WebView sections below no longer apply.
> The Cargo-side posture (Windows-only, WASAPI, secrets in `config.json`) still
> holds. A fresh audit of the Slint stack is TODO. Kept as the historical record.

## Cargo (Rust deps): `cargo audit`

**Scanned 626 crate dependencies. 0 actual vulnerabilities. 17 unmaintained warnings.**

### Unmaintained crates (acceptable)

10× **gtk-rs GTK3 bindings** family (`atk`, `atk-sys`, `gdk`, `gdk-sys`, `gdkwayland-sys`, `gdkx11`, `gdkx11-sys`, `gtk`, `gtk-sys`, `gtk3-macros`):
- All Linux-only deps pulled in transitively by Tauri's cross-platform features
- suflyor is **Windows-only** (uses WASAPI directly, doesn't ship a Linux binary)
- These crates never execute in our release MSI
- Status: ignored

6× **proc-macro-error + unic-***:
- Transitive deps from build-time proc-macros
- Compile-time only, never in the running binary
- Status: ignored

1× **Unsoundness in `glib::VariantStrIter`** (RUSTSEC-2024-0429):
- Linux-only (glib is part of gtk3 chain above)
- Status: not applicable — Windows builds don't link glib

### Recommendations

None for v0.0.11. If a future version targets Linux (we don't), gtk-rs 0.18 should be replaced with gtk4 bindings (0.7+). For Windows-only operation as of today: 0 actionable findings.

## NPM (frontend deps): `npm audit`

```
found 0 vulnerabilities
```

Clean. Tauri 2 + React 19 + Vite 7 + react-markdown 10 + remark-gfm 4 are all current.

## Manual security review (code-level)

### Capability split (Tauri 2)

- `capabilities/default.json` — overlay window only. Has `core:default`, `core:window:*` (with explicit `allow-start-dragging`), `global-shortcut:*`, `opener:*`, `core:event:default`.
- `capabilities/tile.json` — tile-* windows. Narrow: `core:default`, `core:window:default + allow-hide/show/close/start-dragging`, `core:event:default`. **No** opener, **no** global-shortcut, **no** set-position/size.

Why: tile windows render AI-generated markdown that could include strings sourced from interviewer transcript or scraped web pages. Capability split + assert_overlay guard means a poisoned tile cannot:
- Read or modify config (assert_overlay rejects `get_config` / `save_config` calls)
- Take screenshots (assert_overlay rejects `take_screenshot`)
- Capture audio (assert_overlay rejects `start_session` / `manual_ask_*`)
- Open external URLs (no opener permission)
- Register global hotkeys (no global-shortcut)
- Move/resize windows (no set-position/size)

### `assert_overlay(window)` guard

Applied to 25 sensitive commands. Tested live: tile calls to e.g. `export_config` return `permission denied: this command is restricted to the overlay window (caller=tile-xxx)`.

### Secrets handling

- `config.json` contains `groq_api_key` + `ai_bearer` in plain text. Read/write only via `assert_overlay`-protected commands. File system permissions inherit user-only access from `%APPDATA%\overlay-mvp\`.
- Export (full) keeps secrets — user's responsibility not to share the file. Documented.
- Export (share) blanks 6 sensitive fields via `blank_share_secrets()` pure fn (10 unit tests verify each field gets blanked).
- DevTools is debug-only (release build excludes the auto-open call). Dev mode is explicitly warned about in CLAUDE.md.

### Path traversal defense

`import_config(path)` canonicalises path + verifies it's under Desktop OR Documents. Rejects everything else. Same pattern for `load_session(path)` (must be under sessions dir).

### Plaintext HTTP warning

When `ai_base_url` starts with `http://` AND host is NOT loopback (`127.0.0.1` / `localhost` / `[::1]` / `::1`), Settings shows a yellow chip: "Plaintext HTTP to non-localhost ({host}) — bearer token + prompts travel in clear. Use https:// (Caddy/Nginx in front) for any non-localhost deployment."

Backend doesn't block plaintext — user might be on a LAN-only setup where it's deliberate. UI nudges, doesn't enforce. Loopback case (v0.0.15+) is suppressed because traffic never leaves the machine — no actual exposure.

### Diagnostic dump (v0.0.15+)

Settings → 🆙 Обновления → 📊 Диагностический дамп writes a single sanitized markdown to Desktop for bug reports. Includes:
- Sanitized config (uses `blank_share_secrets` — same 10-test-covered fn as Export-share)
- App version + OS/arch
- Last 50 lines of latest session journal — runs through `sanitize_diagnostic_text` which redacts `gsk_*`, `Bearer *`, `sk-*` token patterns (5 unit tests cover each). The journal output flags that `ai_request` events still contain `system_prompt`/`user_prompt` (which embed `meeting_context`) — user reviews before sharing
- Crash report content (if present) — same `sanitize_diagnostic_text` redaction applied

### Markdown XSS

TileWindow renders AI responses via ReactMarkdown + remark-gfm. Default config sanitises — no `<script>`, no inline event handlers, no raw HTML. `<form>` and `<input>` are NOT rendered from markdown. Tile Esc-to-close listener can't be hijacked by content (no input focus to capture).

### Single-instance lock

`tauri-plugin-single-instance` prevents two concurrent overlay-mvp processes. Without this, the second instance silently fails to register global hotkeys (held by the first), and audio capture races between them. Live regression caught in 2026-05-25.

## Conclusions

Personal-use app, but no actionable security issues found in the v0.0.15 codebase. Re-audit:
- Annually
- After any major dependency bump
- After any change touching `assert_overlay`-protected commands
- After adding any new Tauri command (must consider whether it needs the guard)
