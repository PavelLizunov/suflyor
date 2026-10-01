---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4e9c0dd67702"
source_path: "overlay-backend/src/update.rs"
batch_id: "B09"
total_lines: 374
symbols_count: 11
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "b5fb00beb755db92a0342be44b40f9a644a677448d17a275fb9ff56dec1e7be5"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/update.rs`

- **Batch:** B09
- **Physical Lines:** 374
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `UpdateInfo` | L28 | pub |
| struct | `GhRelease` | L40 | private |
| struct | `GhAsset` | L48 | private |

## Symbols & Routines (11)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `parse_ver` | L61 | `fn parse_ver(s: &str) -> (u64, u64, u64, u8)` |
| function | `version_gt` | L76 | `fn version_gt(a: &str, b: &str) -> bool` |
| function | `pick_installer_url` | L86 | `fn pick_installer_url(assets: &[GhAsset]) -> String` |
| function | `check_latest` | L100 | `fn check_latest(current_version: &str) -> Result<UpdateInfo>` |
| function | `is_trusted_download` | L132 | `fn is_trusted_download(url: &str) -> bool` |
| function | `expected_sha256_for` | L170 | `fn expected_sha256_for(client: &reqwest::Client, url: &str) -> Result<Option<String>>` |
| function | `download_installer` | L194 | `fn download_installer(url: &str) -> Result<PathBuf>` |
| function | `run_installer` | L265 | `fn run_installer(path: &Path) -> Result<()>` |
| function | `version_compare` | L277 | `fn version_compare() -> ()` |
| function | `untrusted_download_host_rejected` | L289 | `fn untrusted_download_host_rejected() -> ()` |
| function | `installer_asset_requires_exact_name` | L345 | `fn installer_asset_requires_exact_name() -> ()` |
