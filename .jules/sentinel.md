## 2026-08-30 - Digest Pre-hashing for Constant-Time Token Comparison
**Vulnerability:** `constant_time_eq` short-circuited when byte slice lengths differed during Authorization header validation in `overlay-backend/src/bridge.rs`, exposing token length via timing side-channels.
**Learning:** Using `constant_time_eq` directly on variable-length secret strings still leaks secret token length when length mismatch causes an early exit.
**Prevention:** Hash variable-length tokens with a cryptographically secure hash function (such as SHA-256) to produce fixed-size byte digests before invoking `constant_time_eq`.
