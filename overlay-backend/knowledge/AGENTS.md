# Knowledge Base Content Authoring (`overlay-backend/knowledge/`)

Authoring guidelines for embedded markdown files (`glossary.md`, `commands.md`, `patterns.md`).

## Compile-Time Embedding

- Embedded into the `overlay-backend` binary at compile time via `include_str!` inside `overlay-backend/src/kb.rs`.
- **Recompilation required:** Any edit to a `.md` file requires a Rust build/test step for changes to take effect in the binary.
- **Preamble handling:** Text prior to the first `## ` line in each file is treated as a preamble comment and stripped during parsing.
- **Empty body skipping:** Entries without body text under their heading are skipped.

## Heading and Alias Format

- **Heading format:** `## <key> [— description or full name]`
  - `key` is extracted as the first whitespace-separated token before `—` and lowercased (e.g. `## Exasol — in-memory ...` yields key `exasol`, `## kubernetes — k8s` yields key `kubernetes`).
  - Keys must be unique within `glossary.md` to avoid definition shadowing.
- **Aliases format:** `Aliases: alias1, alias2, alias3`
  - Optional `Aliases:` or `aliases:` prefix line placed inside the entry body.
  - Used for curated mis-spellings, STT voice variants, Cyrillic transliterations, or alternate names (e.g. `Aliases: starrox, стар рокс`, `Aliases: экзасол`).
  - Parsed out of displayed/injected `body` text into the entry's private `aliases` list.

## Search & RAG (Grounding) Coupling

- **Search (`kb::search`):**
  - Powers the inline search palette (F4) over pre-lowercased cached fields (<5ms response time).
  - Search query is clamped to 200 characters to prevent DoS.
  - Scored ranking order:
    1. Exact key match
    2. Key prefix match (`starts_with`)
    3. Heading substring match
    4. Body substring match
- **RAG Grounding (`kb::reference_for`):**
  - Tokenizes prompt text into whole words (alphanumeric + `-`/`_`, length >= 2).
  - If a prompt token matches an entry's exact `key` or curated `aliases`, the full definition is formatted into a `### Heading\nBody` grounding block for the LLM context.
  - Alias matching requires alias tokens >= 4 chars for single words, or all sub-words present for multi-word aliases.
  - Output is capped by `max_entries` and `max_chars` parameters.

## Content Verification

Run tests to verify structure, floors, and syntax validity:

```pwsh
cargo test --manifest-path overlay-backend/Cargo.toml --lib kb::tests
```

Automated invariant assertions enforced by `kb::tests`:
- **Floor counts:** Total entries >= 1500 (`glossary` >= 1000, `commands` >= 100, `patterns` >= 100).
- **Well-formedness:** Keys, headings, and bodies must be non-empty; keys must be lowercased and trimmed.
- **Uniqueness:** Glossary keys must not collide or shadow earlier definitions.
- **Grounding correctness:** Grounding and alias matching behavior validated by tests.
