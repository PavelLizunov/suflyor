## 2026-09-03 - Plaintext Credential Leak in URL Masking Helpers
**Vulnerability:** `mask_host` parsed non-bracketed URL ports using `authority.rfind(':')` without stripping userinfo (`user:pass@`). When a URL contained embedded credentials (e.g. `http://user:secret@192.168.0.142/v1`), `rfind(':')` matched the userinfo delimiter and returned `:secret@192.168.0.142` as the "port", leaking the password and host verbatim into HTTP logs and diagnostic reports.
**Learning:** Naive URL parsing helpers that use string searching (like `rfind(':')`) instead of proper URL component splitting can mistake credential delimiters in authority fields for port numbers, completely exposing secrets intended to be masked.
**Prevention:** Always strip userinfo (everything before `@` in the authority component) before parsing host and port, and validate that non-bracketed port suffixes consist exclusively of ASCII digits.

## 2026-07-04 - Cross-Platform Home Directory Log Redaction
**Vulnerability:** `redact_user_home` in `diagnostics.rs` only inspected `%USERPROFILE%`, leaving user home directory paths and OS usernames unredacted in exported logs on macOS (`/Users/<username>`) or when `%USERPROFILE%` is missing.
**Learning:** Checking only platform-specific environment variables for log sanitization risks unmasked privacy leaks when porting or running under non-standard shells.
**Prevention:** Always sanitize user paths against `USERPROFILE`, `HOME`, and `dirs::home_dir()` to guarantee complete cross-platform redaction.
