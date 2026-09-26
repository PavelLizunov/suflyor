---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_a47913f2d4e8"
source_path: "overlay-backend/src/kb.rs"
batch_id: "B08"
total_lines: 503
symbols_count: 19
review_state: validated
---

# File Map: `overlay-backend/src/kb.rs`

- **Batch:** B08
- **Physical Lines:** 503
- **Coverage:** 503/503 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `KBEntry` | L34 | pub |

## Symbols & Routines (19)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `new` | L57 | `fn new(key: String, heading: String, body: String, source: &'static str) -> Self` |
| function | `all` | L76 | `fn all() -> &'static [KBEntry]` |
| function | `parse` | L91 | `fn parse(md: &str, source: &'static str) -> Vec<KBEntry>` |
| function | `search` | L164 | `fn search(query: &str, limit: usize) -> Vec<KBEntry>` |
| function | `get` | L229 | `fn get(key: &str) -> Option<&'static KBEntry>` |
| function | `reference_for` | L240 | `fn reference_for(query: &str, max_entries: usize, max_chars: usize) -> Option<String>` |
| function | `reference_for_grounds_domain_terms` | L293 | `fn reference_for_grounds_domain_terms() -> ()` |
| function | `reference_for_matches_aliases` | L307 | `fn reference_for_matches_aliases() -> ()` |
| function | `all_loads_three_sources_with_floors` | L352 | `fn all_loads_three_sources_with_floors() -> ()` |
| function | `every_entry_well_formed` | L375 | `fn every_entry_well_formed() -> ()` |
| function | `search_exact_key_wins` | L391 | `fn search_exact_key_wins() -> ()` |
| function | `search_prefix_beats_body_substring` | L402 | `fn search_prefix_beats_body_substring() -> ()` |
| function | `get_known_unknown` | L415 | `fn get_known_unknown() -> ()` |
| function | `search_empty_query_returns_empty` | L423 | `fn search_empty_query_returns_empty() -> ()` |
| function | `search_respects_limit` | L431 | `fn search_respects_limit() -> ()` |
| function | `heading_lower_and_body_lower_populated_at_parse` | L440 | `fn heading_lower_and_body_lower_populated_at_parse() -> ()` |
| function | `search_truncates_oversized_query` | L461 | `fn search_truncates_oversized_query() -> ()` |
| function | `search_normal_query_works_unchanged` | L481 | `fn search_normal_query_works_unchanged() -> ()` |
| function | `no_duplicate_keys_within_glossary` | L490 | `fn no_duplicate_keys_within_glossary() -> ()` |
