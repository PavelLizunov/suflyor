## 2026-08-01 - Atomic File Creation for POSIX Credentials
**Vulnerability:** Non-atomic file permission assignment when storing sensitive credentials on POSIX systems (`credentials.json` created with default umask before calling `fs::set_permissions`).
**Learning:** Using `fs::write` followed by `fs::set_permissions` leaves a window where unprivileged local users might read sensitive bearer tokens if umask is permissive.
**Prevention:** Use `OpenOptionsExt::mode(0o600)` on Unix platforms to ensure files containing secrets are created atomically with restricted permissions.
