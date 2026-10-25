## 2026-09-08 - Plaintext URL / Credential Leak in reqwest Error Log Formatting
**Vulnerability:** Logging raw `reqwest::Error` instances via `{e:#}` in STT error handlers printed the full request URL into `overlay-host.log`. For HTTP endpoints with embedded credentials (`http://user:secret@host/v1`) or private LAN hostnames, transport failures leaked secrets into the shareable log file.
**Learning:** `reqwest::Error`'s `Display` / `Debug` representation (`{e:#}`) embeds the target URL. Formatting `reqwest::Error` directly in log calls bypasses URL/credential redaction rules.
**Prevention:** Always log high-level transport failure categories (e.g. `transport_failure_kind(&e)`) rather than formatting raw `reqwest::Error` chains in log calls.

## 2026-09-04 - URL Scheme Case Sensitivity Redaction Bypass
**Vulnerability:** `redact_urls` in `diagnostics.rs` searched for `"http://"` and `"https://"` case-sensitively. When log output or URL strings used uppercase or mixed-case schemes (e.g. `HTTP://user:secret@192.168.0.142/v1` or `Https://bridge.internal/v1`), the search missed the URL prefix, leaving embedded user credentials (`user:secret@`) and hostnames unmasked in exported log files and clipboard diagnostic reports.
**Learning:** URL scheme matching in redaction filters must be case-insensitive per RFC 3986. Case-sensitive substring searching allows non-canonical URL schemes to completely bypass security redaction passes.
**Prevention:** Always perform case-insensitive scheme matching (`rest.to_ascii_lowercase()`) when searching for URL boundaries to ensure all scheme variants are redacted.

## 2026-09-03 - Plaintext Credential Leak in URL Masking Helpers
**Vulnerability:** `mask_host` parsed non-bracketed URL ports using `authority.rfind(':')` without stripping userinfo (`user:pass@`). When a URL contained embedded credentials (e.g. `http://user:secret@192.168.0.142/v1`), `rfind(':')` matched the userinfo delimiter and returned `:secret@192.168.0.142` as the "port", leaking the password and host verbatim into HTTP logs and diagnostic reports.
**Learning:** Naive URL parsing helpers that use string searching (like `rfind(':')`) instead of proper URL component splitting can mistake credential delimiters in authority fields for port numbers, completely exposing secrets intended to be masked.
**Prevention:** Always strip userinfo (everything before `@` in the authority component) before parsing host and port, and validate that non-bracketed port suffixes consist exclusively of ASCII digits.

## 2026-09-06 - Plaintext API Keys and Bearer Tokens in Diagnostic Log Exports
**Vulnerability:** `collect_redacted_log` and `build_diag_report` in `diagnostics.rs` masked hostnames, IP addresses, and user home paths, but did not sanitize credential token patterns (`Bearer <token>`, `gsk_<token>`, `sk-<token>`). Exported diagnostic logs on Desktop (`suflyor-log.txt`) could contain raw API keys or Authorization headers verbatim.
**Learning:** Diagnostic export sanitization must explicitly strip known credential token patterns in addition to infrastructure host/IP and user path redaction.
**Prevention:** Always pipe log and diagnostic string exports through `redact_secrets` token-pattern masking before writing to files or the clipboard.

## 2026-07-04 - Cross-Platform Home Directory Log Redaction
**Vulnerability:** `redact_user_home` in `diagnostics.rs` only inspected `%USERPROFILE%`, leaving user home directory paths and OS usernames unredacted in exported logs on macOS (`/Users/<username>`) or when `%USERPROFILE%` is missing.
**Learning:** Checking only platform-specific environment variables for log sanitization risks unmasked privacy leaks when porting or running under non-standard shells.
**Prevention:** Always sanitize user paths against `USERPROFILE`, `HOME`, and `dirs::home_dir()` to guarantee complete cross-platform redaction.

## 2026-10-01 - World-Readable Config File Permissions on POSIX
**Vulnerability:** `save()` in `config.rs` saved `config.json` via default `std::fs::write`, which on Unix/POSIX targets created files subject to default umask permissions (`0644`/`0664`), leaving plain-text secrets and bearer tokens in `config.json` readable by other local system users.
**Learning:** While `credentials.json` had explicit `0o600` permissions on POSIX, `config.json` also holds sensitive API keys and tokens (`ai_bearer`, `groq_api_key`, `hermes_bridge_token`) but relied on default file creation options.
**Prevention:** Always enforce owner-only permissions (`0o600`) when creating temporary files before atomic renames for any file containing sensitive API keys or credentials on POSIX platforms.

## 2026-10-10 - World-Readable Summary & Debrief Sidecar File Permissions on POSIX
**Vulnerability:** `save_in()` and `save_debrief_in()` in `conspect.rs` created `conspects/` (`*.json`) and `debriefs/` (`*.txt`) sidecars via default `std::fs::create_dir_all` and `std::fs::write`, leaving meeting transcripts, summaries, and coaching notes subject to default umask permissions (`0644`/`0755`) on Unix/POSIX targets.
**Learning:** While `sessions/` journals and `config.json` enforced owner-restricted file permissions (`0o600`/`0o700`), sidecar files holding meeting transcripts and AI summaries relied on standard `std::fs` calls, exposing sensitive meeting content to other local system users.
**Prevention:** Always use `crate::paths::ensure_private_dir` and `crate::paths::write_private` when creating directories or writing files under the data root to enforce owner-only (`0o600`/`0o700`) permissions on POSIX targets.

## 2026-10-15 - World-Readable Log and Lock File Permissions on POSIX
**Vulnerability:** `init()` in `logging.rs`, `collect_redacted_log()` in `diagnostics.rs`, and `try_acquire()` in `lifecycle.rs` created `overlay-host.log`, `suflyor-log.txt`, and `suflyor-overlay-singleton.lock` via default `std::fs::OpenOptions` and `std::fs::write`, leaving diagnostic log files and singleton lock files subject to default umask permissions (`0644`/`0664`) on Unix/POSIX targets.
**Learning:** While `sessions/` journals and `config.json` enforced owner-restricted file permissions (`0o600`), application diagnostic logs and exported log summaries in `slint-experiment` relied on standard `std::fs` options, exposing log contents and lock files to other local system users.
**Prevention:** Always use `overlay_backend::paths::open_private_append` and `overlay_backend::paths::write_private` when creating or writing log files, exports, or lock files on POSIX targets.

## 2026-10-20 - World-Readable Audio Recording and Data Directory Permissions on POSIX
**Vulnerability:** `SessionRecorder::start_in`, `Store::open`, `logging::init`, and `settings_controller.rs` created raw session audio recording folders (`recordings/<session_id>/`), catalog database directories, and the application data root via standard `std::fs::create_dir_all`. On Unix/POSIX targets, this left directory permissions subject to default umask (`0755`/`0775`), exposing raw audio recordings (`mic.wav`, `system.wav`) and data files to other local system users.
**Learning:** Directory creation across recorder, logging, and storage modules must explicitly enforce owner-only permissions (`0o700`) rather than relying on default `std::fs::create_dir_all`.
**Prevention:** Always use `overlay_backend::paths::ensure_private_dir` when creating application data root and audio recording directories on POSIX targets.

## 2026-10-20 - World-Readable SQLite Catalog Database Permissions on POSIX
**Vulnerability:** `Store::open()` in `sqlite_store.rs` created `catalog.sqlite` via `rusqlite::Connection::open` and `std::fs::create_dir_all`, leaving SQLite database files containing sensitive session histories, transcripts, AI turns, and curated personal memory subject to default umask permissions (`0644`/`0755`) on Unix/POSIX targets.
**Learning:** While `sessions/` journals and `config.json` enforced restricted permissions, `Store::open()` created SQLite catalog files without explicit permission restriction, exposing sensitive session transcripts and personal memory to other local system users.
**Prevention:** Always use `crate::paths::ensure_private_dir` for database directories and call `crate::paths::restrict_to_owner(path)` immediately after opening/creating SQLite database files on POSIX targets.
