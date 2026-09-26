---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_cd8ade15fd6c"
source_path: "overlay-backend/src/session_names.rs"
batch_id: "B08"
total_lines: 165
symbols_count: 12
review_state: validated
---

# File Map: `overlay-backend/src/session_names.rs`

- **Batch:** B08
- **Physical Lines:** 165
- **Coverage:** 165/165 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `Entry` | L23 | private |

## Symbols & Routines (12)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `file_path_in` | L31 | `fn file_path_in(root: &Path) -> PathBuf` |
| function | `load_in` | L35 | `fn load_in(root: &Path) -> Map` |
| function | `write_in` | L42 | `fn write_in(root: &Path, map: &Map) -> std::io::Result<()>` |
| function | `prune` | L51 | `fn prune(map: &mut Map) -> ()` |
| function | `set_in` | L67 | `fn set_in(root: &Path, session_id: &str, name: &str, ts: u128) -> std::io::Result<()>` |
| function | `get_in` | L86 | `fn get_in(root: &Path, session_id: &str) -> Option<String>` |
| function | `set` | L95 | `fn set(session_id: &str, name: &str, ts: u128) -> ()` |
| function | `get` | L105 | `fn get(session_id: &str) -> Option<String>` |
| function | `set_then_get_round_trips` | L120 | `fn set_then_get_round_trips() -> ()` |
| function | `empty_id_is_noop` | L133 | `fn empty_id_is_noop() -> ()` |
| function | `missing_file_loads_empty` | L142 | `fn missing_file_loads_empty() -> ()` |
| function | `prune_keeps_newest_by_ts` | L148 | `fn prune_keeps_newest_by_ts() -> ()` |
