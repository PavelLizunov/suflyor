---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_71a93e035ed7"
source_path: "overlay-backend/src/persistence/maintenance.rs"
batch_id: "B08"
total_lines: 757
symbols_count: 33
review_state: validated
---

# File Map: `overlay-backend/src/persistence/maintenance.rs`

- **Batch:** B08
- **Physical Lines:** 757
- **Coverage:** 757/757 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `DbHealth` | L18 | pub |
| struct | `ClearResult` | L391 | pub |

## Symbols & Routines (33)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `diagnose_and_repair_default` | L37 | `fn diagnose_and_repair_default() -> Result<DbHealth>` |
| function | `check_default` | L48 | `fn check_default() -> Result<DbHealth>` |
| function | `diagnose_and_repair_at` | L82 | `fn diagnose_and_repair_at(path: &Path, backups: &Path) -> Result<DbHealth>` |
| function | `open_main` | L148 | `fn open_main(path: &Path) -> Result<Connection>` |
| function | `run_checks` | L159 | `fn run_checks(conn: &Connection) -> Vec<String>` |
| function | `repair` | L215 | `fn repair(conn: &Connection, actions: &mut Vec<String>) -> ()` |
| function | `fts5_tables` | L249 | `fn fts5_tables(conn: &Connection) -> Vec<String>` |
| function | `quote_ident` | L272 | `fn quote_ident(name: &str) -> String` |
| function | `backup_before_repair` | L281 | `fn backup_before_repair(path: &Path, backups: &Path) -> Option<String>` |
| function | `prune_backups` | L324 | `fn prune_backups(backups: &Path, keep: usize) -> ()` |
| function | `is_backup_file` | L359 | `fn is_backup_file(p: &Path) -> bool` |
| function | `quote_string` | L374 | `fn quote_string(s: &str) -> String` |
| function | `clear_memory_candidates_default` | L402 | `fn clear_memory_candidates_default() -> Result<ClearResult>` |
| function | `clear_memory_items_default` | L415 | `fn clear_memory_items_default() -> Result<ClearResult>` |
| function | `count_memory_candidates_default` | L424 | `fn count_memory_candidates_default() -> Result<usize>` |
| function | `count_memory_items_default` | L433 | `fn count_memory_items_default() -> Result<usize>` |
| function | `clear_table_at` | L445 | `fn clear_table_at(path: &Path, backups: &Path, table: &str) -> Result<ClearResult>` |
| function | `count_table_at` | L486 | `fn count_table_at(path: &Path, table: &str) -> Result<usize>` |
| function | `default_catalog_path` | L503 | `fn default_catalog_path() -> Result<PathBuf>` |
| function | `backups_dir` | L511 | `fn backups_dir() -> Result<PathBuf>` |
| function | `seed_db` | L524 | `fn seed_db(path: &Path) -> ()` |
| function | `row_count` | L548 | `fn row_count(path: &Path) -> i64` |
| function | `repair_is_healthy_backs_up_and_preserves_rows` | L555 | `fn repair_is_healthy_backs_up_and_preserves_rows() -> ()` |
| function | `missing_db_is_healthy_noop` | L594 | `fn missing_db_is_healthy_noop() -> ()` |
| function | `prune_keeps_at_most_five_backups` | L605 | `fn prune_keeps_at_most_five_backups() -> ()` |
| function | `seed_memory_db` | L641 | `fn seed_memory_db(path: &Path) -> ()` |
| function | `count_of` | L666 | `fn count_of(path: &Path, table: &str) -> i64` |
| function | `clear_candidates_clears_only_the_queue_and_backs_up` | L673 | `fn clear_candidates_clears_only_the_queue_and_backs_up() -> ()` |
| function | `clear_items_clears_only_curated_memory` | L694 | `fn clear_items_clears_only_curated_memory() -> ()` |
| function | `clear_rejects_non_whitelisted_table` | L710 | `fn clear_rejects_non_whitelisted_table() -> ()` |
| function | `clear_missing_db_is_noop` | L728 | `fn clear_missing_db_is_noop() -> ()` |
| function | `count_reads_default_rows` | L738 | `fn count_reads_default_rows() -> ()` |
| function | `is_backup_file_matches_only_our_pattern` | L750 | `fn is_backup_file_matches_only_our_pattern() -> ()` |
