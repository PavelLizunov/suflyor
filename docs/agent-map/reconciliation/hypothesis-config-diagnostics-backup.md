# Original config C03/C04: bounded diagnostics readiness embedding and backup file permissions evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_config_diagnostics_backup_confirmed.py) inspect frozen configuration diagnostics and persistence sources. They do not write live configuration files, mutate permissions, or query live networks. C03 and C04 remain confirmed mechanisms.

## C03 — unmasked URLs and machine directory paths in diagnostics and preview

In `config.rs`, [readiness](<../../../overlay-backend/src/config.rs#L957-L1069>) builds status details by formatting raw `ep.base_url` (`format!("{} · {} · {}", provider, ep.base_url, ep.model)`), raw `stt_gigaam_dir` (`format!("gigaam · {}", self.stt_gigaam_dir)`), and raw `stt_whisper_url` (`format!("whisper · {}", self.stt_whisper_url)`) without applying `mask_host` or filesystem path sanitization.
Similarly, in [preview_server_settings](<../../../overlay-backend/src/config.rs#L1940-L2027>), raw endpoint URLs and local paths (`gigaam_dir_current`, `gigaam_dir_incoming`) are copied directly into the preview struct without authority masking.

## C04 — backup file permissions and unredacted broken config preservation

In [save_to_path](<../../../overlay-backend/src/config.rs#L1561-L1620>), atomic replacement writes live updates to `.json.tmp` with `.mode(0o600)` under `#[cfg(unix)]`. However, when saving the undo backup `.json.bak`, it calls `std::fs::write(&bak, redacted)` without explicit `0o600` permission mode flags.
In [preserve_corrupt_config](<../../../overlay-backend/src/config.rs#L1390-L1401>), unparseable config files are renamed to `config.json.broken-<unix_secs>` directly via `std::fs::rename`. The preserved broken files retain whatever raw secret tokens were written in them without redaction and without a retention pruning cap.

## Limits

No live configuration files were corrupted on disk, no permission bits were altered on host operating systems, and no secret keys were transmitted. Original statuses in `candidates.json` remain `confirmed`.
