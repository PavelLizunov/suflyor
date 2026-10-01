---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_313da15f08cb"
source_path: "overlay-backend/src/credentials.rs"
batch_id: "B09"
total_lines: 282
symbols_count: 16
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "ca55678cf2413befb067551ccee562d09821cd1f5419769fa161172a6b1340d2"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/credentials.rs`

- **Batch:** B09
- **Physical Lines:** 282
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `SecretSlot` | L12 | pub |

## Symbols & Routines (16)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `target` | L19 | `fn target(self) -> &'static str` |
| function | `wide` | L28 | `fn wide(value: &str) -> Vec<u16>` |
| function | `write` | L33 | `fn write(slot: SecretSlot, secret: &str) -> Result<()>` |
| function | `read` | L66 | `fn read(slot: SecretSlot) -> Result<Option<String>>` |
| function | `delete` | L102 | `fn delete(slot: SecretSlot) -> Result<()>` |
| function | `ensure_dir_permissions` | L124 | `fn ensure_dir_permissions(dir: &Path) -> Result<()>` |
| function | `credentials_path` | L140 | `fn credentials_path() -> Result<PathBuf>` |
| function | `read_map` | L148 | `fn read_map() -> HashMap<String, String>` |
| function | `write_to_path` | L158 | `fn write_to_path(path: &Path, bytes: &[u8]) -> Result<()>` |
| function | `write_map` | L195 | `fn write_map(map: &HashMap<String, String>) -> Result<()>` |
| function | `write` | L201 | `fn write(slot: SecretSlot, secret: &str) -> Result<()>` |
| function | `read` | L212 | `fn read(slot: SecretSlot) -> Result<Option<String>>` |
| function | `delete` | L217 | `fn delete(slot: SecretSlot) -> Result<()>` |
| function | `provider_slots_are_stable_and_distinct` | L234 | `fn provider_slots_are_stable_and_distinct() -> ()` |
| function | `posix_credentials_are_written_with_mode_0600` | L242 | `fn posix_credentials_are_written_with_mode_0600() -> ()` |
| function | `posix_credentials_directory_has_mode_0700` | L268 | `fn posix_credentials_directory_has_mode_0700() -> ()` |
