# Embedded KB, palette and reference search: source-linked contract

**Evidence:** principal source/callers inspected at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. No Cargo/Slint/live keyboard or performance benchmark executed. Embedded KB, user snippets and archived transcript FTS are different stores and interfaces.

## Embedded data and parser

[KBEntry/all](../../../overlay-backend/src/kb.rs#L19-L64) compile three Markdown sources with `include_str!`: glossary, commands and patterns. `all()` initializes an in-memory vector once; [parse](../../../overlay-backend/src/kb.rs#L74-L150) splits sections on `\n## ` headings, extracts backtick key or lowercased heading, precomputes lowercase text and curated aliases. It does not construct a persistent vector index; the module's inverted-index/performance comment is not established by the actual vector scan.

Embedded [knowledge inputs](../../../overlay-backend/knowledge/AGENTS.md) are compiled code inputs, not routine documentation. Their changes require affected backend checks even though extension is Markdown. This distinction is correctly present in GitHub classifier and missing in current native suffix-only docs filter.

## Palette versus grounding

[search](../../../overlay-backend/src/kb.rs#L164-L225) trims query, clamps 200 Unicode characters, lowercases and scans entries into exact-key, prefix-key, heading-substring and body-substring buckets. Empty query or limit=0 returns no rows. Ranking is deterministic declaration order inside bucket, not BM25/embedding semantic similarity; text byte lengths are used for contains checks.

[F4 open_palette](../../../slint-experiment/src/bin/overlay_host/aux_windows/help_palette.rs#L82-L128) attempts initial `kb::search("",20)` despite comment promising top/popular entries. Because search rejects empty query, actual initial list is empty. [query callback](../../../slint-experiment/src/bin/overlay_host/aux_windows/help_palette.rs#L111-L119) updates up to 20 hits synchronously; [activation](../../../slint-experiment/src/bin/overlay_host/aux_windows/help_palette.rs#L158-L185) resolves full entry by exact key and opens a read-only content tile, closing palette.

[get](../../../overlay-backend/src/kb.rs#L227-L232) resolves exact normalized key. [reference_for](../../../overlay-backend/src/kb.rs#L234-L286) separately matches whole query tokens or all tokens of curated aliases, not palette substring rank. It includes up to requested count and checks `String::len()` before including a whole block: parameter named max_chars is enforced as UTF-8 **bytes** here. Oversized blocks are skipped instead of truncated.

[AI build_request](../../../overlay-backend/src/ai/prompt.rs#L34-L60) combines recent transcript/question for KB grounding with three entries and 4000-byte block budget. This is prompt reference content, not external command execution. [MLX compact auto-tile prompt](../../../overlay-backend/src/runtime/trigger_detect.rs#L126-L193) has its own selected reference/short response policy; source framing does not prove correct model understanding.

## Separate user snippets and archive FTS

[Config snippet defaults](../../../overlay-backend/src/config/snippets.rs#L1-L25) are a user-overridable data pack, distinct from embedded KB entries. Default literal presence does not itself prove every snippet is reachable in current UI. Reviewed F4 palette calls kb::search/get, not config.snippets. Production host/backend source search found snippet data/default/transfer preservation references, but no active snippet-dispatch caller in these current Slint paths; historical `/key` snippet behavior is not established by saved defaults alone.

[Archive FTS search](../../../overlay-backend/src/persistence/sqlite_store.rs#L395-L436) queries SQLite FTS5 utterance/question/answer index with BM25 and snippet output; it is not the embedded KB. [Archive input adapter](../../../slint-experiment/src/bin/overlay_host/aux_windows/archive.rs#L929-L947) converts alphanumeric terms to prefix tokens; embedded KB search is direct case-insensitive scan instead.

## Verification boundaries

[KB tests](../../../overlay-backend/src/kb.rs#L288-L503) declare source floors, key/alias boundaries and search expectations, but were not natively executed. Source-seam research tests can validate empty-search and byte-budget implementation without claiming search correctness/performance. Live acceptance needs F4 empty/query/activation, Russian aliases, long Unicode paste, exact key full body, compiled-data revision and screenshots/focus/stealth; <5ms and ~1600/~2000 counts in historical prose are not measured metrics of this run.
