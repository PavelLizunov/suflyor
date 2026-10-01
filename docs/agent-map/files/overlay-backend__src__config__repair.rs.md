---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_a79d2c877dfe"
source_path: "overlay-backend/src/config/repair.rs"
batch_id: "B09"
total_lines: 183
symbols_count: 10
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "e5240dc0990c8641a2310b2fa538ede9e62721beca5e836d0970c02c91e445a2"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/config/repair.rs`

- **Batch:** B09
- **Physical Lines:** 183
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (10)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `cp1252_char_to_byte` | L19 | `fn cp1252_char_to_byte(c: char) -> Option<u8>` |
| function | `repair_cp1252_mojibake` | L69 | `fn repair_cp1252_mojibake(s: &str) -> Option<String>` |
| function | `byte_to_cp1252_char` | L100 | `fn byte_to_cp1252_char(b: u8) -> char` |
| function | `corrupt` | L135 | `fn corrupt(s: &str) -> String` |
| function | `repairs_real_mojibake` | L140 | `fn repairs_real_mojibake() -> ()` |
| function | `leaves_clean_russian_untouched` | L153 | `fn leaves_clean_russian_untouched() -> ()` |
| function | `leaves_ascii_untouched` | L158 | `fn leaves_ascii_untouched() -> ()` |
| function | `leaves_empty_untouched` | L163 | `fn leaves_empty_untouched() -> ()` |
| function | `leaves_legit_latin1_untouched` | L168 | `fn leaves_legit_latin1_untouched() -> ()` |
| function | `idempotent_on_already_clean` | L175 | `fn idempotent_on_already_clean() -> ()` |
