---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_12f364d37dad"
source_path: "overlay-backend/src/ocr.rs"
batch_id: "B10"
total_lines: 270
symbols_count: 12
review_state: validated
---

# File Map: `overlay-backend/src/ocr.rs`

- **Batch:** B10
- **Physical Lines:** 270
- **Coverage:** 270/270 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (12)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `tesseract_root` | L52 | `fn tesseract_root() -> Option<PathBuf>` |
| function | `tesseract_exe` | L79 | `fn tesseract_exe() -> Option<PathBuf>` |
| function | `is_available` | L86 | `fn is_available() -> bool` |
| function | `run_ocr` | L96 | `fn run_ocr(bgra: &[u8], width: u32, height: u32, lang: &str) -> Result<String>` |
| function | `spawn_tesseract` | L145 | `fn spawn_tesseract(exe: &Path, tessdata: &Path, lang: &str) -> std::io::Result<Child>` |
| function | `normalize_ocr_text` | L158 | `fn normalize_ocr_text(s: &str) -> String` |
| function | `bgra_to_bmp` | L171 | `fn bgra_to_bmp(bgra: &[u8], width: u32, height: u32) -> Vec<u8>` |
| function | `bmp_header_is_well_formed_and_top_down` | L224 | `fn bmp_header_is_well_formed_and_top_down() -> ()` |
| function | `normalize_strips_formfeed_trailing_ws_and_blank_edges` | L248 | `fn normalize_strips_formfeed_trailing_ws_and_blank_edges() -> ()` |
| function | `normalize_keeps_interior_blank_lines_and_order` | L254 | `fn normalize_keeps_interior_blank_lines_and_order() -> ()` |
| function | `run_ocr_rejects_empty_or_short_buffer` | L260 | `fn run_ocr_rejects_empty_or_short_buffer() -> ()` |
| function | `default_lang_is_rus_plus_eng` | L267 | `fn default_lang_is_rus_plus_eng() -> ()` |

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L125
