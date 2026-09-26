# O6: AI Dataflow, Context Assembly, Memory & Token Budgeting

## 1. Prompt Construction & Pipeline (`overlay-backend/src/ai/`)
- **Decomposed Architecture**: 11 dedicated submodules manage protocol polymorphism, completion retries, SSE streaming, concurrency control, and prompt assembly.
- **Four Distinct Prompt Paths**:
  - `Path A (Live Ask / F9)`: Combines system role, user profile, KB reference grounding (up to 3 entries / 4000 chars), anti-injection guards, and tagged transcript lines (`[Mic]`/`[System]`).
  - `Path B (Auto-Tiles / Trigger Detect)`: Standard and MLX-compact profiles with strict formatting constraints (60–120 words).
  - `Path C (Meeting Summary)`: Map-reduce architecture splitting long transcripts into bounded chunks that fit the active model's context window.
  - `Path D (Debrief)`: Speech coach evaluation operating strictly on mic-only transcript audio.

## 2. Endpoint Resolution: Local vs. Cloud Multiplexing
- **`Config::ai_endpoint(prep: bool)`**: Central routing multiplexer in `overlay-backend/src/config.rs:746-828`:
  - `mlx`: Interacts with macOS MLX local runtime.
  - `local`: Direct loopback HTTP bridge (default `127.0.0.1:8080/v1`).
  - `openai` / `anthropic`: Cloud models authenticating via credentials stored in OS secure storage (Windows Credential Manager).
  - `codex`: Native Codex app-server subscription bridge.
  - `cloud`: Cloud bridge via `ai_base_url` + `ai_bearer`.
- **Deep Lock Guard**: Prevents managed local model endpoints from being triggered or reloaded when deep lock mode is active.

## 3. Context Assembly & Multi-Tier Layering
1. **Layer 1 (Audio Transcript)**: Structured `TranscriptLine` entries tagged by audio channel (`Mic` vs. `System`).
2. **Layer 2 (Knowledge Base)**: ~1600 embedded glossary and command entries. Whole-word token matching prevents substring collisions (e.g. "java" vs. "javascript").
3. **Layer 3 (Personal Memory)**: Approved entries from `catalog.sqlite`. Symmetrically root-matched to handle Russian declensions and diminutives. Capped at 8 items / 1200 characters to prevent prompt bloat.
4. **Layer 4 (Meeting Context / Profile)**: User-defined background information framed explicitly as background data rather than overriding instructions.

## 4. Streaming, Markdown & UI Reactivity
- **SSE Stream Processing**: Chunk-based SSE parsing in `stream.rs` handling `OpenAiCompatible`, `OpenAiResponses`, and `AnthropicMessages` delta formats.
- **50ms UI Throttling**: Stream deltas are accumulated and throttled to 50ms intervals before dispatching `slint::invoke_from_event_loop`. This drops ~80% of transient UI repaints, preventing UI stutter.
- **Incremental Markdown Rendering**: `pulldown-cmark` generates structured AST blocks. Streaming mode suppresses trailing unclosed LaTeX/math delimiters to prevent flickering.

## 5. Security, Redaction & Privacy
- **Defense in Depth Against IP Leaks**:
  - `http_log.rs`: Logs only HTTP status and byte size, never URL, query parameters, or response body text.
  - Error tiles always display generic, localized error strings ("Не удалось получить ответ от AI"). The raw exception chain is never rendered in screen-visible UI tiles.
  - Local LAN IP addresses and port numbers are sanitized from all logs and diagnostics bundles.
- **Anti-Prompt-Injection**: System prompts instruct the LLM that transcript contents are passive data, not executable instructions. Memory retrieval filters 12 explicit injection patterns.
