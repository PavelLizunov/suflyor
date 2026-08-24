## 2026-06-27 - Atomic Secure Mode on POSIX Credential File Creation
**Vulnerability:** Standard `fs::write` creates files with default umask permissions (e.g. `0644`), creating a race condition where sensitive API credentials in `credentials.json` are world-readable before `fs::set_permissions(..., 0o600)` is invoked.
**Learning:** Calling `fs::set_permissions` after `fs::write` leaves a window where secrets exist on disk with unprivileged read access.
**Prevention:** Use `std::fs::OpenOptions` with `.mode(0o600)` on Unix via `OpenOptionsExt` so that credential files are created with restricted permissions atomically from the first byte written.
