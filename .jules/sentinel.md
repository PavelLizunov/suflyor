## 2026-07-04 - Cross-Platform Home Directory Log Redaction
**Vulnerability:** `redact_user_home` in `diagnostics.rs` only inspected `%USERPROFILE%`, leaving user home directory paths and OS usernames unredacted in exported logs on macOS (`/Users/<username>`) or when `%USERPROFILE%` is missing.
**Learning:** Checking only platform-specific environment variables for log sanitization risks unmasked privacy leaks when porting or running under non-standard shells.
**Prevention:** Always sanitize user paths against `USERPROFILE`, `HOME`, and `dirs::home_dir()` to guarantee complete cross-platform redaction.

## 2026-07-09 - URL Authority Userinfo Credential Leak in Log Redaction
**Vulnerability:** `mask_host` in `config.rs` searched for `:port` using `rfind(':')` over the full URL authority string. In URLs containing basic auth credentials (`http://user:password@host/path`), `rfind(':')` matched the colon inside `user:password`, leaking `:password@host` in redacted logs and diagnostic previews.
**Learning:** Naive URL string splitting for redaction can expose embedded credentials when host/port parsing does not strip `userinfo@` (`user:pass@`) first or validate port digits.
**Prevention:** Always strip `userinfo@` before extracting URL host and port, and strictly validate that port strings consist of ASCII digits.
