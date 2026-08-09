# Codex subscription connection (RC15)

RC15 integrates ChatGPT subscription sign-in through the official local
[`codex app-server`](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
stdio JSON-RPC protocol. Suflyor initializes one short-lived child process per
account action and uses only these stable methods:

- `account/read`
- `account/login/start` with `type: chatgptDeviceCode`
- `account/logout`
- `account/login/completed` notifications

The child receives an isolated `CODEX_HOME`, runs in an empty working directory,
and is forced to `cli_auth_credentials_store="keyring"`. The official Codex
process owns token persistence and refresh. Suflyor never reads, imports,
exports, logs, or parses Codex credentials or `auth.json`.

## Why RC15 is connect-only

The stable app-server `thread/start` / `turn/start` surface exposes an agent and
documents shell, file-edit, MCP, app, web-search, and other tool events. A
read-only sandbox prevents writes but still grants filesystem reads; approval
policy controls approvals rather than removing tools. The stable protocol does
not currently document an enforceable empty tool set plus zero filesystem
authority for an inference turn.

Consequently `ai_provider = "codex"` resolves to the explicit
`CodexSubscriptionConnectOnly` protocol. It never falls through to the custom
bridge or another paid provider. Live answers remain disabled until an official
stable protocol can enforce both boundaries. At that point the adapter may map
`item/agentMessage/delta` and `turn/completed` into the existing `AiEvent`
stream, using ephemeral threads in a non-user working directory.

OpenCode and Qwen are not fabricated as provider types. They can use the
existing Custom OpenAI-compatible bridge only when they actually expose a
compatible inference endpoint.
