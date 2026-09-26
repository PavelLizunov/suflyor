---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_75e14fb627ce"
source_path: "suflyor-teratts/src/manifest.rs"
batch_id: "B12"
total_lines: 243
symbols_count: 12
review_state: validated
---

# File Map: `suflyor-teratts/src/manifest.rs`

- **Batch:** B12
- **Physical Lines:** 243
- **Coverage:** 243/243 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `Manifest` | L21 | pub |
| struct | `ManifestFile` | L27 | pub |

## Symbols & Routines (12)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `pinned` | L37 | `fn pinned() -> Result<Manifest>` |
| function | `from_json` | L41 | `fn from_json(raw: &str) -> Result<Manifest>` |
| function | `release_dir` | L72 | `fn release_dir(&self, tts_root: &Path) -> PathBuf` |
| function | `check_installed` | L80 | `fn check_installed(manifest: &Manifest, release_dir: &Path) -> Result<()>` |
| function | `installed_voices` | L100 | `fn installed_voices(release_dir: &Path) -> Vec<String>` |
| function | `pinned_manifest_parses_and_is_immutable` | L129 | `fn pinned_manifest_parses_and_is_immutable() -> ()` |
| function | `pinned_manifest_covers_all_ten_voices` | L150 | `fn pinned_manifest_covers_all_ten_voices() -> ()` |
| function | `eng_m4_duration_style_keeps_the_verified_digest` | L175 | `fn eng_m4_duration_style_keeps_the_verified_digest() -> ()` |
| function | `release_dir_names_the_revision` | L189 | `fn release_dir_names_the_revision() -> ()` |
| function | `rejects_unsafe_or_unpinned_manifests` | L196 | `fn rejects_unsafe_or_unpinned_manifests() -> ()` |
| function | `installed_check_requires_marker_and_sizes` | L209 | `fn installed_check_requires_marker_and_sizes() -> ()` |
| function | `voice_discovery_needs_both_style_files` | L232 | `fn voice_discovery_needs_both_style_files() -> ()` |
