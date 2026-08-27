## 2026-07-10 - Constant-time Bearer Token Validation via SHA-256 Hashing
**Vulnerability:** Comparing secret bearer tokens directly using naive byte slice comparisons or early-returning length checks in `constant_time_eq` exposes token length and character match timing side-channels.
**Learning:** Checking byte slice equality with `a.len() != b.len()` early-returns before performing bitwise XOR, allowing attackers to measure request duration differences to determine token length.
**Prevention:** Always digest secret token inputs with a fixed-length cryptographic hash function (such as SHA-256) before performing constant-time bitwise comparisons. This guarantees fixed-length 32-byte constant-time execution regardless of input string length or content.
