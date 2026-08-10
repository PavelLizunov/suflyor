# RC16: real Codex subscription provider

Status: implemented on the isolated RC16 branch, based on RC15 (`531d6fb`).

## Outcome

After an explicit ChatGPT sign-in, Settings shows the Codex models available to
that account, remembers the selected model, shows bounded subscription-usage
state, and routes text answers through the official local `codex app-server`.
No OpenAI API key, Hermes OAuth client, direct ChatGPT backend, or credential
file parsing is introduced.

## Official experimental protocol used

The installed official Codex app-server experimental schema exposes the
required fail-closed surface:

- `account/read`, `account/login/start`, `account/logout` (existing RC15 flow);
- paginated `model/list` (`data[].model`, `displayName`, `isDefault`,
  `inputModalities`, `nextCursor`);
- `account/rateLimits/read` plus optional `account/rateLimits/updated`;
- `thread/start`, `turn/start`, `item/agentMessage/delta`,
  `thread/tokenUsage/updated`, `turn/completed`, and `turn/interrupt`.

Initialization explicitly opts into `experimentalApi`. Inference also requires
`permissionProfile/list`, `environments: []`, `dynamicTools: []`,
`allowProviderModelFallback: false`, and exact model pinning on both
`thread/start` and `turn/start`. If any field/profile is unavailable or rejected,
there is no inference and no fallback.

## Security contract

Keep every RC15 boundary: isolated `CODEX_HOME`, empty dedicated working
directory, an allowlisted child environment, Windows Credential Manager storage
owned by Codex, no `auth.json` read/write, no token logging/export, no automatic
browser launch, and no raw app-server error text in UI or support logs.

Every turn is ephemeral with `approvalPolicy: "never"`, the app-owned
`suflyor-text-only` permission profile (`:root = "deny"`), elevated Windows
sandbox enforcement, network access false, exactly one empty runtime
workspace root, no instruction sources, no selected capability roots, no
environment, no dynamic tools, web disabled, empty MCP configuration, and no
inherited shell environment. This denies host-file and command-network access;
unexpected tool attempts are interrupted as defense in depth. Only text input
and matching `agentMessage`/safe reasoning lifecycle events are accepted. Any
server request, tool/file/command/MCP/web/
image/collaboration/permission surface, wrong id, model reroute, active-model
mismatch, nonempty workspace, or protocol drift interrupts/drops the response
and shows a generic security error. Suflyor never invokes
`thread/shellCommand/*` or `process/*` RPC.

## Smallest implementation map

### 1. Backend app-server adapter

Extend `overlay-backend/src/codex_subscription.rs`; do not add another HTTP
client or copy Hermes transport code.

- Split the existing stdio request/response plumbing from account-only logic so
  one initialized child can run account, catalog, rate-limit, or turn requests.
- Add a bounded `ProviderSnapshot` that reads account state, all visible model
  pages (hard cap on pages/items, reject duplicate exact ids), and a sanitized usage
  summary. Never surface server descriptions, emails, raw errors, paths, or
  account ids.
- Add one-turn inference: start an ephemeral thread in the existing empty
  workspace, flatten the bounded role history into one text `turn/start` input,
  emit one
  `AiEvent::Start`, forward only matching `item/agentMessage/delta`, capture the
  matching token-usage update, and finish on the matching `turn/completed`.
- On receiver close, timeout, process exit, mismatched terminal state, server
  request/tool approval, or malformed data: send `turn/interrupt` when possible,
  kill/reap the child, and emit exactly one generic terminal error.
- Keep one short-lived process per operation for RC16. A persistent daemon is
  deferred until measured startup latency justifies lifecycle complexity.

Pure parsers/builders get unit tests for pagination, model validation/default
selection, id correlation, delta ordering, token usage, completion statuses,
timeout, cancellation, tool-request rejection, and error redaction.

### 2. AI routing and accounting

In `overlay-backend/src/ai.rs` replace
`CodexSubscriptionConnectOnly` with a live Codex protocol and branch to the
stdio adapter before the existing reqwest/SSE path. `overlay-backend/src/ai/provider.rs`
must remain HTTP-only; its Codex arms should disappear rather than simulate an
HTTP endpoint.

- `stream_chat_endpoint` delegates Codex work through `spawn_blocking` and the
  existing `AiEvent` receiver contract.
- `complete_with_usage_endpoint` aggregates the same adapter for non-streaming
  re-ask, auto-tile, debrief, and structuring paths and returns captured token
  usage when available.
- Codex subscription traffic has zero dollar cost in suflyor accounting through
  the endpoint's protocol-level `is_unmetered()` decision; Codex model ids are
  never priced as OpenAI API calls.
- `slint-experiment/src/bin/overlay_host/tile_routes.rs` must treat Codex auth as
  app-server-owned and must not reject F9 merely because `AiEndpoint.bearer` is
  empty. Add route tests for normal F9 and Shift+F9.
- `Config::readiness()` stays config-only: Codex is ready when a selected model
  is present; live account health remains the Settings snapshot.

### 3. Persistence and import/export

Add additive `#[serde(default)] pub codex_model: String` in
`overlay-backend/src/config.rs`. No schema-version bump is needed.

- `Config::ai_endpoint()` resolves Codex with empty URL/bearer and this model.
- Successful catalog refresh keeps the saved model when still available;
  otherwise it chooses the server's visible default (or first visible model),
  persists that exact id, and shows a generic fallback status.
- Copy the non-secret preference in `merge_server_settings` and include the
  model id in the redacted preview. Never import/export account material.
- Extend `overlay-backend/src/config/tests.rs` for legacy defaults, round-trip,
  endpoint resolution, preview, and server-settings merge.

### 4. Dynamic Settings UI

In `slint-experiment/ui/settings_panel.slint` replace the RC15 connect-only
warning with a real model row and subscription usage text. Add:

- `codex-models`, `codex-model-index`, `codex-models-busy`;
- transient `codex-models-status` and `codex-usage-status`;
- `codex-model-selected(string)` and `codex-models-refresh()` callbacks.

Use the existing `SettingsComboBox` pattern. Show validated model ids (not a
hard-coded catalog), preserve the current saved id while loading, disable the
picker during refresh, and keep Connect/Disconnect/device-code actions intact.
Refresh catalog/usage after signed-in status, provider selection, explicit
refresh, and Settings reopen. Generation-gate every result with the existing
`CODEX_UI_GENERATION` so provider/language switches cannot repaint stale data.

Wire this in `slint-experiment/src/bin/overlay_host/settings_ai.rs` and seed/reset
it in `settings_controller.rs::populate_token_status`. Every new Slint string is
English `@tr(...)` with a matching Russian pair in
`translations/ru/LC_MESSAGES/slint-replay.po`; use ASCII status markers only.
`settings_reset_guard` must see both new `*-status` properties reset on reopen.

### 5. Mock and automated tests

Do not use a real account in tests. Use deterministic in-memory app-server wire
transcripts and pure request/response builders for initialize, account,
paginated catalog, rate limits, thread/turn, deltas, usage, completion,
malformed responses, wrong ids, nonempty workspace, and tool requests. They
never open a browser or perform network access.

Backend tests exercise the fixture directly. A debug/winbrat harness may place
a compiled mock `codex.exe` first on `PATH` on a VM without official Codex; no
production executable override is needed. The fixture never opens a browser or
performs network access.

Extend/replace `slint-experiment/tests/codex_copy_guard.rs` with guards for the
dynamic picker, selected-model save callback, no connection-only copy, account
refresh generation gating, and EN/RU catalog entries. Keep the copy-code tests.

## Verification gates

1. Focused backend/config/Slint guard tests with the mock only.
2. Full `scripts/ci.ps1` green on the exact branch head.
3. Winbrat debug `ui-mcp` build using the mock: Settings at 1280x768 and
   720x600, EN and RU, signed-out/signed-in/loading/empty/error states, model
   selection persistence, usage state, F9 delta stream, cancellation, and no
   clipping/stale state.
4. Winbrat mock request log confirms isolated cwd/home, ephemeral thread,
   read-only sandbox, approval never, selected model, interrupt on cancel, and
   no direct endpoint/token/client id.
5. Owner visual acceptance using `docs/retest-v0.36.1-rc.16.html`.

## Explicitly out of scope

- Hermes/OpenCode/Qwen OAuth or client ids; direct ChatGPT backend calls;
- reading/copying Codex credentials; real-login automation in CI or winbrat;
- reasoning-effort/service-tier/personality pickers;
- persistent app-server daemon or cross-tile native Codex thread reuse;
- stable release, tag, direct master publication, or real-login automation.
