# Original installers C03/C04: bounded network protocol and staging evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_installer_protocol_staging_hypotheses.py) inspect frozen download, update, and installer sources. They do not initiate live network requests, download files, or spawn processes. C03 and C04 remain hypotheses.

## C03 — download protocols, redirect behavior, and host filtering

[curl_download](<../../../overlay-backend/src/download.rs#L18-L48>) verifies that the input URL string starts with `https://`, then invokes system `curl` with flags `-L` (follow redirects), `--fail`, `--retry 5`, `--retry-all-errors`. It does not specify `--proto =https`, `--proto-redir =https`, `--max-redirs`, or a destination IP/host restriction.
In `update.rs`, [is_trusted_download](<../../../overlay-backend/src/update.rs#L132-L157>) restricts updater URLs to `https` scheme, default port 443, no userinfo, and hosts `github.com/{REPO}/*`, `objects.githubusercontent.com`, and `release-assets.githubusercontent.com`.
However, [download_installer](<../../../overlay-backend/src/update.rs#L198-L202>) builds `reqwest::Client` with default redirect policy (following up to 10 redirects) without explicit `redirect(Policy::none())` or redirect target re-verification against `is_trusted_download`.

## C04 — staging directory, atomic rename, and marker validation

In `teratts_install.rs`, [install_with](<../../../overlay-backend/src/teratts_install.rs#L190-L275>) implements staging into `<dir>.staging`, checks files against pinned digests, writes `manifest.json` as a publication marker, and atomically promotes the directory via `fs::rename`. An invalid install is moved to `<dir>.broken-*` quarantine.
In contrast, [ocr_install](<../../../overlay-backend/src/ocr_install.rs#L30-L79>) downloads into the root directory directly, extracts in-place, and only removes the target directory `tesseract` if [dest_has_engine](<../../../overlay-backend/src/ocr_install.rs#L85-L93>) returns false. In `update.rs`, [download_installer](<../../../overlay-backend/src/update.rs#L213-L225>) enforces a 100,000-byte minimum and initial `MZ` header check before saving to `%TEMP%\suflyor-update\suflyor-slint-setup.exe`.

## Limits

No malicious redirect was followed, no network packets were captured, and no partial installer execution was performed. Original statuses in `candidates.json` remain `hypothesis`.
