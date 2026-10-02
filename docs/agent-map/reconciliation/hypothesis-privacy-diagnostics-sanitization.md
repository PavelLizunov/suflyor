# Original privacy C01/C02: bounded home directory and URL log sanitization evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_privacy_diagnostics_sanitization_confirmed.py) inspect frozen diagnostics report formatting and log redaction logic. They do not read private user log files or write to user desktop folders. C01 and C02 remain confirmed mechanisms.

## C01 — home directory and username string masking forms

In `diagnostics.rs`, [redact_home_all_forms](<../../../slint-experiment/src/bin/overlay_host/diagnostics.rs#L216-L220>) replaces user directory paths by invoking `redact_home_in` across three explicit separator variations:
1. raw literal `home` (e.g. `C:\Users\alice`);
2. doubled backslashes `home.replace('\\', "\\\\")` (e.g. `C:\\Users\\alice`);
3. forward slashes `home.replace('\\', "/")` (e.g. `C:/Users/alice`).
As explicitly noted in the source documentation, 8.3 short names (`X3D_MU~1`) or the raw username token outside of the full home directory prefix are not matched or redacted.

## C02 — URL host masking vs non-HTTP scheme bypass

In `diagnostics.rs`, [redact_urls](<../../../slint-experiment/src/bin/overlay_host/diagnostics.rs#L117-L140>) searches case-insensitively for `http://` and `https://` prefixes, passing matched URL spans up to the first whitespace into `overlay_backend::config::mask_host`.
Non-HTTP URLs (such as `ws://`, `wss://`, `ftp://`, `file://`) and bare hostnames without a scheme are not matched by `redact_urls`.
In [collect_redacted_log](<../../../slint-experiment/src/bin/overlay_host/diagnostics.rs#L732-L742>), sanitization executes in fixed nested order: `redact_secrets(&redact_user_home(&redact_ipv4(&redact_urls(&raw))))`.

## Limits

No real user log files or desktop directories were read or modified, and no unmasked diagnostic telemetry was broadcast. Original statuses in `candidates.json` remain `confirmed`.
