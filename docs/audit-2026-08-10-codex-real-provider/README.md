# RC16 Codex provider UI audit

Scope: the Settings > AI providers Codex account/model surface and the F9
answer tile on Winbrat. All captures use a synthetic `codex.exe` fixture, a
synthetic account/model catalog, and an empty isolated provider directory. No
real OAuth, credentials, account inference, endpoint, or user transcript was
used.

## Environment

- Branch: `codex/codex-real-provider-rc16`
- Version: `v0.36.1-rc.16`
- VM: Winbrat, 1280x768 desktop
- Renderer: Slint `winit-software`
- Final QA binary: debug `ui-mcp`, `SLINT_EMIT_DEBUG_INFO=1`, SHA-256
  `8D38C386835E15480E5E81C3524C036E8F51E71B4EB325B60BEB4AB948D5963E`
- Required viewport checks: 1200x640 and 720x600 Settings windows, English and
  Russian

## Evidence

- `before-empty-ru-720.png`: pre-fix empty-catalog state that incorrectly
  claimed an account fallback was selected.
- `after-models-en-1200.png`: signed-in account, usage summary, and selected
  synthetic model at wide width.
- `after-models-ru-720.png`: Russian compact-width layout.
- `after-loading-ru-720.png`: refresh-in-progress state with picker and actions
  disabled.
- `after-empty-ru-720.png`: final truthful empty-catalog state.
- `streaming-f9-ru.png`: mock Codex delta rendered in an F9 tile.
- `security-denial-ru.png`: unexpected command approval request rejected with a
  generic error.
- `cancelled-f9-ru.png`: partial answer preserved after the receiver is stopped.

The corresponding synthetic wire log (kept outside the repository) verifies
the selected model, ephemeral thread, approval `never`, the app-owned
`suflyor-text-only` profile, elevated Windows sandbox, root filesystem deny,
network deny, empty MCP/dynamic tools/environments/capability roots, isolated
home/workspace, and `turn/interrupt` cleanup. It contains no direct endpoint,
token, or OAuth client id.
