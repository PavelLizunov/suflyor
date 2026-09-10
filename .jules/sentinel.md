## 2026-09-08 - Plaintext URL / Credential Leak in reqwest Error Log Formatting
**Vulnerability:** Logging raw `reqwest::Error` instances via `{e:#}` in STT error handlers printed the full request URL into `overlay-host.log`. For HTTP endpoints with embedded credentials (`http://user:secret@host/v1`) or private LAN hostnames, transport failures leaked secrets into the shareable log file.
**Learning:** `reqwest::Error`'s `Display` / `Debug` representation (`{e:#}`) embeds the target URL. Formatting `reqwest::Error` directly in log calls bypasses URL/credential redaction rules.
**Prevention:** Always log high-level transport failure categories (e.g. `transport_failure_kind(&e)`) rather than formatting raw `reqwest::Error` chains in log calls.

## 2026-09-03 - Plaintext Credential Leak in URL Masking Helpers
**Vulnerability:** `mask_host` parsed non-bracketed URL ports using `authority.rfind(':')` without stripping userinfo (`user:pass@`). When a URL contained embedded credentials (e.g. `http://user:secret@192.168.0.142/v1`), `rfind(':')` matched the userinfo delimiter and returned `:secret@192.168.0.142` as the "port", leaking the password and host verbatim into HTTP logs and diagnostic reports.
**Learning:** Naive URL parsing helpers that use string searching (like `rfind(':')`) instead of proper URL component splitting can mistake credential delimiters in authority fields for port numbers, completely exposing secrets intended to be masked.
**Prevention:** Always strip userinfo (everything before `@` in the authority component) before parsing host and port, and validate that non-bracketed port suffixes consist exclusively of ASCII digits.

## 2026-07-04 - Cross-Platform Home Directory Log Redaction
**Vulnerability:** `redact_user_home` in `diagnostics.rs` only inspected `%USERPROFILE%`, leaving user home directory paths and OS usernames unredacted in exported logs on macOS (`/Users/<username>`) or when `%USERPROFILE%` is missing.
**Learning:** Checking only platform-specific environment variables for log sanitization risks unmasked privacy leaks when porting or running under non-standard shells.
**Prevention:** Always sanitize user paths against `USERPROFILE`, `HOME`, and `dirs::home_dir()` to guarantee complete cross-platform redaction.
