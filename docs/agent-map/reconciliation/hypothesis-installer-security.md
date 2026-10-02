# Original installers C01/C02: bounded extraction and TOCTOU evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_installer_security_hypotheses.py) inspect frozen installer and download source. They do not run actual downloads, spawn installers, or execute tar extraction. C01 and C02 remain hypotheses.

## C01 — archive extraction parameters and destination containment

[extract_tar_bz2](<../../../overlay-backend/src/download.rs#L85-L105>) invokes `tar.exe` / `tar` with `-xf <tarball> -C <dest_dir>`. The invocation does not pass explicit `--exclude`, `--no-same-permissions`, or canonicalize path containment before running. [ocr_install](<../../../overlay-backend/src/ocr_install.rs#L30-L75>) extracts directly into the `%APPDATA%\suflyor` root directory relying on archive members to be prefixed with `tesseract/`.

## C02 — path-based verify-then-extract / write-then-spawn window

[verify_sha256](<../../../overlay-backend/src/download.rs#L106-L125>) reads bytes from `&Path`, calculates the digest, and returns `Ok(())` without keeping an open locked file handle or passing an in-memory buffer to the extractor. `ocr_install` then calls `extract_tar_bz2` on the path. In `update.rs`, [download_installer](<../../../overlay-backend/src/update.rs#L194-L245>) verifies bytes in-memory and writes them to `%TEMP%\suflyor-update\suflyor-slint-setup.exe`. Later, [run_installer](<../../../overlay-backend/src/update.rs#L265-L273>) executes the file via `Command::new(path).spawn()` without re-verifying the digest.

## Limits

No malicious tarball was unpacked, no filesystem race was injected, and no native updater execution was performed. Original statuses in `candidates.json` remain `hypothesis`.
