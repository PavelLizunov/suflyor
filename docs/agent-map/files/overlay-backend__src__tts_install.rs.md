---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_afa5d1e8e11a"
source_path: "overlay-backend/src/tts_install.rs"
batch_id: "B10"
total_lines: 248
symbols_count: 7
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "af7d577c73caf3dd9439005d353d88ff4b92bdf0c25ef13bd8ff2121556985ac"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/tts_install.rs`

- **Batch:** B10
- **Physical Lines:** 248
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `VoiceProgress` | L82 | pub |
| struct | `VoicePack` | L21 | pub |

## Symbols & Routines (7)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `label_for` | L39 | `fn label_for(&self, ru: bool) -> &'static str` |
| function | `relocalize_voice_labels` | L70 | `fn relocalize_voice_labels(label: &str, ru: bool) -> String` |
| function | `tts_dir` | L94 | `fn tts_dir() -> Option<PathBuf>` |
| function | `pack_installed` | L99 | `fn pack_installed(dir: &Path) -> bool` |
| function | `any_voice_installed` | L114 | `fn any_voice_installed() -> bool` |
| function | `packs_have_valid_pins` | L205 | `fn packs_have_valid_pins() -> ()` |
| function | `pack_labels_follow_ui_language` | L220 | `fn pack_labels_follow_ui_language() -> ()` |
