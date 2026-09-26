---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_afa5d1e8e11a"
source_path: "overlay-backend/src/tts_install.rs"
batch_id: "B10"
total_lines: 248
symbols_count: 7
review_state: validated
---

# File Map: `overlay-backend/src/tts_install.rs`

- **Batch:** B10
- **Physical Lines:** 248
- **Coverage:** 248/248 lines (100%)

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
