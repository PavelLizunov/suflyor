# Memory architecture (ADR) — suflyor

Design by fable (2026-07-03), grounded in the code (every "current" claim cited file:line).
**Status: proposed — not yet built.** Owner wanted the full design before implementation.
Supersedes the sketch in `docs/personal-memory-and-session-store-architecture.md` (keeps its
FTS → cosine-in-Rust → sqlite-vec ladder).

Two owner pains this must solve:
1. **Facts come from raw, messy STT dialog** → they must be FORMATTED/normalized, not stored
   verbatim (a captured line like «Даже, даже писана мэру.сь.» currently becomes a permanent
   memory row).
2. **Related facts fragment** (z14-backup: имя / подсеть / IP = 3 rows) and **most memory
   never reaches the model** (recency-capped at newest-8).

## 0. Verified current state
- **Storage** — SQLite `migrations/0003_memory.sql`: `memory_candidates` + `memory_items`
  (`id, profile_id, kind, text, source_session_id, approved_at_ms, archived_at_ms,
  embedding_status`). `embedding_status` defaults `'none'` and is **never set otherwise**
  (`sqlite_store.rs:480,495`) — a reserved-but-dead column. `list_memory_items` orders
  `approved_at_ms DESC` — **recency is the only ranking**.
- **Retrieval → answers** — `memory/context_builder.rs`: `context_for_meeting(base)` takes
  **no query** (can't be relevance-aware), dumps newest 8 approved (`MAX_ITEMS=8`,
  `MAX_BLOCK_CHARS=1200`, `MAX_ITEM_CHARS=240`) as a REFERENCE block. Item #9+ never reaches
  the model. 8 call sites, all with the question + `is_local` in scope.
- **Retrieval → summary** — `memory/summary_ref.rs`: lexical keyword-gate (name-like
  `key_terms` must occur in the transcript; RU-declension prefix-stem), top-5, ≤800 chars,
  framed **decode-only** (`runtime.rs:396-401`). Comment: "embeddings are Phase 4".
- **Fact creation** — heuristic extractor `candidates.rs:39` (Q&A verbatim `"Q:…\nA:…"`, topic
  words) via «Извлечь»; manual ⭐-capture `tile_copy.rs:316` (verbatim note); transcript
  multi-⭐ join `aux_windows.rs` (manual coherence). **Nothing cleans anything between capture
  and storage.**
- **Context / AI** — llama-server `-c 8192 --jinja`, Gemma 4 E4B Q4 (~5 GB) at :8080.
  Codebase already treats 8192 as a hard budget (summary 12k chars local vs 24k cloud;
  map-reduce on overflow; no-think forced to avoid overflow). `ai::complete()/complete_with_usage()`
  = non-streaming JSON-in/out, no-think — **the exact primitive normalization needs, zero new
  plumbing.** FTS5 `unicode61 remove_diacritics 2` already indexes RU+EN (`0002_fts.sql`);
  heavy memory work already runs on worker threads (`settings_memory.rs:105`).

## 1. Principles
1. Garbage must not become memory — raw STT is a *source*, never a stored fact.
2. Decode-only discipline wherever an LLM touches memory + a **deterministic Rust validator**
   (never trust LLM self-restraint alone).
3. Provenance is sacred — keep the verbatim capture forever beside the normalized text.
4. Consent model unchanged — only approved items feed prompts; approved items are never
   silently rewritten (merge is user-confirmed).
5. Retrieval is query-driven + budget-bounded; 0 matches degrades to today's behavior.
6. No new in-process native runtimes (sherpa/onnx sidecar lesson) — model-shaped things that
   aren't already linked run as a separate llama.cpp process.
7. Each phase ships alone; pure-function-first (validator/scorer/merger unit-tested like
   `key_terms`).

## 2. Pipeline (target)
`capture → heuristic pre-clean (Rust) → LLM rewrite (no-think, JSON) → Rust grounding
validator → store (normalized text + verbatim source_text + entity + norm_status) →
dedup/merge-on-write → retrieve (FTS BM25 top-K [+ cosine RRF in Phase 4], recency fallback)
→ inject (labelled [entity] block, "transcript wins" framing)`.

## 3. Normalization / formatting (owner MUST) — hybrid, 3 layers
1. **Heuristic pre-clean (Rust, pure, always, free):** collapse whitespace, strip immediate
   word repeats («Даже, даже»→«Даже,»), strip clause-start fillers (reuse the list already in
   `runtime.rs:1051-1055`), drop dangling ≤2-letter fragments between terminal punctuation
   (the «.сь.» class), normalize punctuation runs. **Never touches fenced code — byte-exact.**
   Ceiling: makes garbage *shorter*, can't resolve pronouns/mishearings → why layer 2 exists.
2. **LLM rewrite (`ai::complete()`, forced no-think):** input = pre-cleaned span + ±2-3
   neighbor transcript lines (to resolve «он»→«Иван», «эта подсеть»→«10.20.0.0/24»). Output =
   strict JSON `{"facts":[{"entity","text"}]}`, one atomic self-contained fact each. Prompt:
   «перепиши как короткий самодостаточный факт, раскрой местоимения, НИЧЕГО не добавляй;
   числа/IP/имена/идентификаторы — дословно; не восстанавливается → пустой список». **Empty
   list is a first-class result** (garbage → no fact). Async/background → capture UX stays the
   instant SQLite write; row upgrades in place. Endpoint = `cfg.ai_endpoint(false)` (same as
   every AI feature; no new egress class, no new toggle). «Извлечь» normalizes a whole
   session's candidates in ONE batched call.
3. **Deterministic grounding validator (Rust, pure — the real guarantee):** accept only if
   every number/IP/Latin-id/ALL-CAPS token in the output appears **verbatim** in span∪context
   (zero tolerance — one wrong digit in an IP is catastrophic); every content word (≥4ch,
   non-stopword, prefix-stemmed) is contained (≤10% novel glue allowed, **0** novel
   identifiers); shape sane (non-empty, ≤~300ch, no headers, len ≤1.5× input); negation words
   preserved both ways. Fail ⇒ store heuristic text, `norm_status='failed'`, keep raw. «мэру.сь.»
   becomes a literal test fixture.

Per-source: ⭐transcript line = layers 1+2; ⭐tile block = trim only (AI answers already clean;
rewriting code = harm); «Извлечь» candidates = 1+2 batched; deep-extract = is the LLM pass;
typed fact = trim only (user authored it; rewriting uninvited breaks consent). Un-normalized
(server down) → `norm_status='heuristic'`, lazily retried on Memory-tab open / next «Извлечь».

## 4. Storage — migration `0005_memory_v2.sql` (additive only)
`memory_items` + `memory_candidates` gain: `source_text` (verbatim; NULL=text is source),
`entity` (lowercase subject key; NULL=none), `norm_status` (`none|heuristic|llm|failed`),
`last_used_at_ms`, `use_count`. New `memory_fts` FTS5 vtable (`body = text + ' ' + entity`,
`unicode61 remove_diacritics 2`, triggers keep it synced, archived rows removed). Phase 4:
`memory_embeddings(item_id, provider, model, dim, vector BLOB, updated_at_ms)`; `embedding_status`
finally used (`none→pending→done`). Legacy rows (`source_text NULL`, `norm_status='none'`) behave
as today; lazy-upgrade opportunistically. No backfill.

## 5. Coherent memory (entity grouping) — the z14-backup fix
- **At extraction (primary):** the planned "deep extract" (3b.2b) — one `complete()` per
  session over the *transcript*, instructed to group all attributes of one object into ONE
  entity-keyed fact → `memory_candidates` for review. Same validator (identifiers
  verbatim-in-transcript). Automates what the manual multi-⭐ join already proved.
- **At write (merge lifecycle — mem0 pattern, not code):** before insert — exact dup → skip;
  same `entity` or content-word Jaccard ≥0.8 → flag candidate `reason="дополняет: <entity>"`,
  offer **merge** on approval (one LLM call combines old∪new, validator applies, provenance
  keeps both). Phase 4: "similar" = cosine ≥ threshold; ADD/UPDATE/NOOP over {new, top-3
  similar} = mem0's lifecycle. Approved items only auto-*flagged*, never silently rewritten.

## 6. Relevance retrieval
- **Phase A — scored lexical, NO embeddings (cheap, high value):**
  `context_for_meeting(base, query, is_local)` (all 8 sites have query+endpoint). One SQL:
  `bm25(memory_fts)` over the query's content words (quoted, OR-joined, ≥5ch as prefix with
  last char dropped — the `term_in_tokens` trick via FTS5 prefix). Top-K by BM25; fill
  remainder newest-first; **0 matches ⇒ exactly today's block (hard floor, can't regress)**.
  `use_count++`, `last_used_at_ms`. Provider-aware caps: 8/1200 local, 12/2400 cloud. Summary
  keeps its (deliberately conservative) keyword-gate.
- **Phase B — embeddings + hybrid (when lexical stalls):** triggers = store >~150-200 items OR
  logged retrieval misses (paraphrase/synonymy, deep RU morphology, RU↔EN). Model =
  **multilingual-e5-small Q8 GGUF (~130 MB, 384-d)** (mind the `query:`/`passage:` prefixes);
  bge-m3 is the upgrade path. Serving = **second llama-server :8082 `--embeddings`** (reuses all
  existing install/launch/job-object/readiness machinery). Store = **f32 BLOB column + cosine
  scan in Rust** (1000 facts ≈ <1ms; ANN pays only at 10⁵-10⁶ — `// ponytail: brute-force,
  sqlite-vec if >~50k`). Fusion = **RRF** (`Σ 1/(60+rank)`; the one algorithm worth borrowing).
  Embed on background at capture; missing/stale-model vector → lexical-only fallback + lazy
  re-embed (model swap self-heals).
- **Rejected in-process:** `fastembed-rs` rides `ort` with rc-version pins that can collide
  with transcribe-rs's `ort` (DirectML STT) → sidecar instead. Cloud embeddings rejected
  (facts are the most private data — local-only vectors).

## 7. Injection
Keep `format_memory_block` shape + (1) per-fact `[entity]` prefix; (2) header line «если факт
из памяти противоречит разговору — приоритет у разговора» (~15 tok; answers-path analog of the
summary decode-only rule); (3) keep the СПРАВКА/«НЕ задание» framing (prevents topic-lock).
Summary block untouched.

## 8. Context-window math (2.2 chars/tok, RU)
Live-ask local: system ~1100 + meeting ≤300 + **memory 1200ch≈550** + transcript ~300-500 +
question ~100 + **4096 output** = **~6700/8192**. The output reservation (not memory) is the
pressure point; current 1200-char cap fits with ~1.5k headroom, and top-K keeps it **constant**
as the store grows. **Don't grow the local memory cap; don't raise `-c` for memory.** Summary
local ≈ 8100/8192 (already at the edge) → memory_ref stays 800ch, growth via relevance only.
Raising `-c` doubles KV-RAM + slows CPU prefill (hurts the CPU-fallback machines) — only a
*transcript*-window feature would justify it; RAG decouples memory from `-c` permanently.

## 9. Ecosystem — borrow patterns, link nothing
mem0 → write-time ADD/UPDATE/NOOP *shape* (ignore its auto-ingest-everything, breaks consent).
LangChain/LlamaIndex → only **RRF** (~15 lines). Rejected crates: `fastembed` (ort collision),
`sqlite-vec` (C ext for a 20-line cosine), `hnsw_rs`/`instant-distance` (ANN for 10³ = negative
value), `rig`/`swiftide` (frameworks replacing ~200 lines with a dep tree). Phase-4 client =
`reqwest` + `serde_json` + one cosine fn — all in-tree.

## 10. Phased plan
| Phase | Contents | Effort | Risk / mitigation |
|---|---|---|---|
| **M1 — Normalization on capture** (the MUST, first) | 0005 migration; `memory::normalize` (heuristic + LLM `complete()` + grounding validator, pure, «мэру.сь.» fixture); wired per §3; review UI shows normalized + raw | **M** ~3-4d | LLM invents → validator+provenance; llama down → heuristic+lazy retry; UX → all async |
| **M2 — Relevance retrieval, no embeddings** | `context_for_meeting(base,query,is_local)` + FTS5 BM25 top-K + recency fallback + use stats + provider caps; 8 sites | **S-M** ~1-2d | FTS MATCH injection → token quoting; floor = today's behavior |
| **M3 — Coherence: deep extract + merge** | LLM session extractor → entity-grouped candidates; exact/entity/Jaccard dedup on write; merge affordance | **M** ~3d | over-merge → user-confirmed for approved; hallucination → validator vs transcript |
| **M4 — Embeddings + hybrid** | e5-small Q8 sidecar :8082; `memory_embeddings`; background embed queue; RRF fusion; model-swap re-embed | **M-L** ~4-5d | +130 MB + 1 process → existing lifecycle; e5 prefix bug → unit-test request builder |
| **M5 — Lifecycle polish** | usage-sorted review, "unused 90d" archive *suggestions* (never auto-delete), count hint | **S** | none material |

**Recommended first release = M1 + M2:** both felt pains, **zero new dependencies, zero new
processes, a hard behavioral floor.**

## 11. Deliberately NOT designed (named re-entry conditions)
Multi-profile memory (column exists, open since 0003); time-decay auto-deletion (curated memory
must not evaporate); vector DB; cloud embeddings; in-process embedder. Each re-enters only on
its stated trigger above.
