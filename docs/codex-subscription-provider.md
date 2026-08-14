# Codex subscription provider (RC16)

RC16 extends the RC15 ChatGPT subscription sign-in through the official local
[`codex app-server`](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
stdio JSON-RPC protocol with account-aware model selection and text inference.
Suflyor initializes one short-lived child process per account or turn action.
It uses the account methods plus the experimental model/turn surface:

- `account/read`
- `account/login/start` with `type: chatgptDeviceCode`
- `account/logout`
- `account/login/completed` notifications
- paginated `model/list` and bounded rate-limit snapshots
- `permissionProfile/list`, `thread/start`, `turn/start`
- matching agent-message/reasoning lifecycle, token usage, completion and
  `turn/interrupt`

The child receives an isolated `CODEX_HOME`, runs in an empty working directory,
and is forced to `cli_auth_credentials_store="keyring"`. The official Codex
process owns token persistence and refresh. Suflyor never reads, imports,
exports, logs, or parses Codex credentials or `auth.json`.

## RC16 security boundary

The built-in read-only profile is not used because it permits local reads.
Every RC16 turn instead requires an app-owned `suflyor-text-only` permission
profile under strict config with elevated Windows sandbox enforcement,
filesystem `:root = "deny"`, and command network disabled. The returned thread
must echo the exact profile, pinned model, approval-never policy, no network,
empty instruction sources, and the one dedicated workspace root or the turn
fails closed.

Selected capability roots, environments and dynamic tools are empty; web, MCP,
model fallback and inherited shell environment are disabled. Suflyor accepts
only text input and matching safe message/reasoning events. Any command, file,
MCP, web or unknown item is interrupted. This is a no-host-files and
no-command-network boundary, not a claim that the model cannot attempt a tool.

`ai_provider = "codex"` resolves only to `CodexSubscription`; it never falls
through to the custom bridge or another paid provider. F9 and Shift+F9 use the
selected account model, and subscription usage is unmetered in suflyor's USD
accounting. Image input is not supported by this provider in RC16.

OpenCode and Qwen are not fabricated as provider types. They can use the
existing Custom OpenAI-compatible bridge only when they actually expose a
compatible inference endpoint.
