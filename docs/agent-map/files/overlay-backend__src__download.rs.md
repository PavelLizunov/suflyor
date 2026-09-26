---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ac24827e0dae"
source_path: "overlay-backend/src/download.rs"
batch_id: "B09"
total_lines: 167
symbols_count: 12
review_state: validated
---

# File Map: `overlay-backend/src/download.rs`

- **Batch:** B09
- **Physical Lines:** 167
- **Coverage:** 167/167 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (12)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `curl_download` | L18 | `fn curl_download(url: &str, dest: &Path) -> Result<()>` |
| function | `system_curl` | L51 | `fn system_curl() -> PathBuf` |
| function | `extract_tar_bz2` | L66 | `fn extract_tar_bz2(tarball: &Path, dest_dir: &Path) -> Result<()>` |
| function | `system_bsdtar` | L81 | `fn system_bsdtar() -> PathBuf` |
| function | `verify_sha256` | L96 | `fn verify_sha256(path: &Path, expected_hex: &str, label: &str) -> Result<()>` |
| function | `hex` | L112 | `fn hex(bytes: &[u8]) -> String` |
| function | `no_window` | L121 | `fn no_window(cmd: &mut Command) -> &mut Command` |
| function | `no_window` | L128 | `fn no_window(cmd: &mut Command) -> &mut Command` |
| function | `hex_is_lowercase_and_padded` | L139 | `fn hex_is_lowercase_and_padded() -> ()` |
| function | `curl_download_rejects_non_https_urls` | L144 | `fn curl_download_rejects_non_https_urls() -> ()` |
| function | `verify_sha256_trims_expected_hash` | L152 | `fn verify_sha256_trims_expected_hash() -> ()` |
| function | `system_curl_is_not_empty` | L164 | `fn system_curl_is_not_empty() -> ()` |
