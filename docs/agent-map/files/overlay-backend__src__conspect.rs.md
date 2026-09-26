---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4d80a1dc46a6"
source_path: "overlay-backend/src/conspect.rs"
batch_id: "B08"
total_lines: 761
symbols_count: 49
review_state: validated
---

# File Map: `overlay-backend/src/conspect.rs`

- **Batch:** B08
- **Physical Lines:** 761
- **Coverage:** 761/761 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `ConspectPart` | L45 | pub |
| struct | `Conspect` | L56 | pub |

## Symbols & Routines (49)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `new` | L87 | `fn new(session_id: String, is_ru: bool, fingerprint: u64, single_pass: bool, sources: Vec<String>,) -> Self` |
| function | `usable_summaries` | L115 | `fn usable_summaries(&self) -> Vec<String>` |
| function | `has_usable_parts` | L129 | `fn has_usable_parts(&self) -> bool` |
| function | `missing_part_indices` | L135 | `fn missing_part_indices(&self) -> Vec<usize>` |
| function | `fingerprint` | L147 | `fn fingerprint(formatted: &str) -> u64` |
| function | `conspects_dir` | L154 | `fn conspects_dir() -> Option<PathBuf>` |
| function | `safe_stem` | L161 | `fn safe_stem(session_id: &str) -> Option<String>` |
| function | `save` | L177 | `fn save(c: &Conspect) -> bool` |
| function | `save_in` | L188 | `fn save_in(dir: &Path, c: &Conspect) -> anyhow::Result<()>` |
| function | `load` | L205 | `fn load(session_id: &str) -> Option<Conspect>` |
| function | `load_in` | L211 | `fn load_in(dir: &Path, session_id: &str) -> Option<Conspect>` |
| function | `replace_latex_command` | L243 | `fn replace_latex_command(s: &str, cmd: &str, sym: &str) -> String` |
| function | `sanitize_summary` | L267 | `fn sanitize_summary(s: &str) -> String` |
| function | `exists` | L318 | `fn exists(session_id: &str) -> bool` |
| function | `exists_in` | L323 | `fn exists_in(dir: &Path, session_id: &str) -> bool` |
| function | `session_ids` | L335 | `fn session_ids() -> std::collections::HashSet<String>` |
| function | `debriefs_dir` | L358 | `fn debriefs_dir() -> Option<PathBuf>` |
| function | `save_debrief` | L364 | `fn save_debrief(session_id: &str, text: &str) -> bool` |
| function | `save_debrief_in` | L368 | `fn save_debrief_in(dir: &Path, session_id: &str, text: &str) -> anyhow::Result<()>` |
| function | `load_debrief` | L383 | `fn load_debrief(session_id: &str) -> Option<String>` |
| function | `load_debrief_in` | L387 | `fn load_debrief_in(dir: &Path, session_id: &str) -> Option<String>` |
| function | `debrief_session_ids` | L395 | `fn debrief_session_ids() -> std::collections::HashSet<String>` |
| function | `delete_debrief` | L415 | `fn delete_debrief(session_id: &str) -> bool` |
| function | `delete_debrief_in` | L423 | `fn delete_debrief_in(dir: &Path, session_id: &str) -> bool` |
| function | `delete` | L433 | `fn delete(session_id: &str) -> bool` |
| function | `delete_in` | L441 | `fn delete_in(dir: &Path, session_id: &str) -> bool` |
| function | `backup` | L457 | `fn backup(session_id: &str) -> bool` |
| function | `backup_in` | L462 | `fn backup_in(dir: &Path, session_id: &str) -> bool` |
| function | `restore_backup` | L472 | `fn restore_backup(session_id: &str) -> ()` |
| function | `restore_backup_in` | L479 | `fn restore_backup_in(dir: &Path, session_id: &str) -> bool` |
| function | `drop_backup` | L488 | `fn drop_backup(session_id: &str) -> ()` |
| function | `drop_backup_in` | L495 | `fn drop_backup_in(dir: &Path, session_id: &str) -> ()` |
| function | `prune_in` | L505 | `fn prune_in(dir: &Path, keep: usize, ext: &str) -> ()` |
| function | `sample` | L538 | `fn sample(id: &str) -> Conspect` |
| function | `save_load_round_trips` | L549 | `fn save_load_round_trips() -> ()` |
| function | `load_missing_is_none` | L560 | `fn load_missing_is_none() -> ()` |
| function | `sanitize_summary_strips_latex_keeps_prices` | L566 | `fn sanitize_summary_strips_latex_keeps_prices() -> ()` |
| function | `load_cleans_legacy_latex_summary` | L588 | `fn load_cleans_legacy_latex_summary() -> ()` |
| function | `debrief_save_load_round_trip` | L602 | `fn debrief_save_load_round_trip() -> ()` |
| function | `delete_debrief_in_removes_txt_and_is_idempotent` | L616 | `fn delete_debrief_in_removes_txt_and_is_idempotent() -> ()` |
| function | `save_debrief_prunes_to_keep_newest` | L632 | `fn save_debrief_prunes_to_keep_newest() -> ()` |
| function | `exists_in_reflects_file_presence` | L649 | `fn exists_in_reflects_file_presence() -> ()` |
| function | `backup_restore_drop_round_trip` | L659 | `fn backup_restore_drop_round_trip() -> ()` |
| function | `usable_summaries_skips_none_and_blank` | L687 | `fn usable_summaries_skips_none_and_blank() -> ()` |
| function | `fingerprint_is_stable_and_distinguishes` | L697 | `fn fingerprint_is_stable_and_distinguishes() -> ()` |
| function | `safe_stem_rejects_traversal_and_separators` | L703 | `fn safe_stem_rejects_traversal_and_separators() -> ()` |
| function | `delete_in_removes_json_and_is_idempotent` | L713 | `fn delete_in_removes_json_and_is_idempotent() -> ()` |
| function | `prune_keeps_newest_n` | L724 | `fn prune_keeps_newest_n() -> ()` |
| function | `incremental_save_overwrites_same_session` | L741 | `fn incremental_save_overwrites_same_session() -> ()` |
