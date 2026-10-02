# Original privacy C03 / settings C10: bounded secret token redaction and token clearing evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_privacy_secrets_token_clear_confirmed.py) inspect frozen diagnostics token redaction and settings token save callbacks. They do not manipulate live API tokens or communicate with external networks. C03 and C10 remain confirmed mechanisms.

## C03 — Secret token prefix matching, missing vendors, and double-space leak bug

In `diagnostics.rs`, [redact_secrets](<../../../slint-experiment/src/bin/overlay_host/diagnostics.rs#L146-L190>) searches specifically for:
- `"Bearer "` (case-sensitive, single space);
- `"gsk_"` (Groq API key prefix);
- `"sk-"` (OpenAI secret key prefix).
It contains no patterns for other common provider tokens (such as `ghp_`, `github_pat_`, `hf_`, `xai-`, `ANTHROPIC_API_KEY`).
Variations such as `"bearer "`, `"BEARER "`, or `"Bearer:\t"` are bypassed without redaction.
Furthermore, if two spaces follow `Bearer` (`"Bearer  <token>"`), the loop strips `"Bearer "`, encounters the second space where `rest.find(char::is_whitespace)` evaluates to `0`, and consumes only the space, leaving the actual secret token unredacted in `rest`.

## C10 — Empty token saves treated as no-ops in Settings UI

In `settings_ai.rs`:
- [on_ai_bearer_save](<../../../slint-experiment/src/bin/overlay_host/settings_ai.rs#L547-L560>) checks `if trimmed.is_empty() { return; }` and skips saving, preventing users from clearing an existing AI bearer token from the UI;
- [on_groq_api_key_save](<../../../slint-experiment/src/bin/overlay_host/settings_ai.rs#L618-L635>) similarly checks `if trimmed.is_empty() { return; }` and aborts the save without erasing the key in configuration.
In [populate_token_status](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L1625-L1635>), the UI displays `[ok] set (N chars)` when a token exists and `[--] not set` when absent, providing no explicit clear button.

## Limits

No live user API credentials were read from disk or modified, and no telemetry was transmitted. Original statuses in `candidates.json` remain `confirmed`.
