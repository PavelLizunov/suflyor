# Original config C06 / audio C05: bounded secret zeroization and macOS system audio recovery evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_secret_zeroization_macos_recovery_hypotheses.py) inspect frozen credentials, config, and macOS audio sources. They do not access the Windows Credential Manager, read live secrets, or run CoreAudio capture. C06 and C05 remain hypotheses.

## C06 — secret zeroization boundaries and String capacity retention

In Windows credential management, [read](<../../../overlay-backend/src/credentials.rs#L66-L100>) reads the `CredentialBlob` bytes into a `Vec<u8>`, converts it into an owned `String::from_utf8(bytes.to_vec())`, and calls `CredFree(raw.cast())` without zeroing the allocated OS blob memory first.
In [write](<../../../overlay-backend/src/credentials.rs#L33-L64>), `blob.fill(0)` is executed directly after `CredWriteW(&credential, 0)`, but is not wrapped in an unwind-safe drop guard (no `ZeroizeOnDrop`).
In `config.rs`, [secret_redacted](<../../../overlay-backend/src/config.rs#L1632-L1643>) clones the configuration (`cfg.clone()`) and calls `.clear()` on secret fields (`ai_bearer`, `groq_api_key`, etc.). Calling `clear()` sets string length to 0 while keeping allocated heap capacity and leaving plaintext in memory until reallocation or drop.

## C05 — macOS system audio route recovery retry loop without exit code stop

In `audio_macos.rs`, [reopen_mic](<../../../overlay-backend/src/audio_macos.rs#L670-L692>) checks for permission errors (`error_code == MIC_START_PERMISSION`) and explicitly stops retry attempts with `RetryResult::Stop`.
In contrast, [reopen_system](<../../../overlay-backend/src/audio_macos.rs#L694-L709>) retries on every non-null controller failure by unconditionally returning `RetryResult::Retry`. It does not inspect `error_code` for terminal device or permission errors, retrying every `ROUTE_RESTART_DELAY` (500 ms) until the global `stop` flag is asserted.

## Limits

No live memory analysis/process dumps were performed, no native Windows Credential Manager calls were made, and no CoreAudio tap disconnections were tested. Original statuses in `candidates.json` remain `hypothesis`.
