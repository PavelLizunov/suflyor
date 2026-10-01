# AI and vision routing: source-linked feature contract

**Evidence:** manually inspected principal source paths at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. No provider login/network turn, screenshots, native builds or account checks executed. This is a bounded routing/protocol map, not full provider/security acceptance.

## Config to protocol

[Config::ai_endpoint](../../../overlay-backend/src/config.rs#L746-L832) resolves active provider and optional prep model into [AiEndpoint](../../../overlay-backend/src/ai/types.rs#L46-L85). A resolved endpoint carries protocol, model, in-memory credential, local flag and optional Codex reasoning effort.

| Provider | Resolved route | Important distinction |
| --- | --- | --- |
| `cloud`/legacy fallback | OpenAI-compatible bridge using `ai_base_url`/`ai_bearer` | Raw bridge fields are not the active endpoint when local/direct providers are selected |
| `local` | OpenAI-compatible HTTP using local URL/bearer/model or prep model | Managed loopback lifecycle and arbitrary external endpoint are different ownership classes |
| `mlx` | Managed MLX model placeholder resolved at send time | Route acquires an owned endpoint/request lease; platform/model/deep-lock checks precede use |
| `openai` | Native OpenAI Responses API | Credentials loaded from provider-specific secure storage API; no direct key in Config |
| `anthropic` | Native Anthropic Messages API | `x-api-key` authorization, separate system block and protocol-specific content parsing |
| `codex` | Official Codex app-server stdio turn | Separate account/security-profile/model-pinned flow; not an HTTP bearer route |

[ai_endpoint_cloud](../../../overlay-backend/src/config.rs#L845-L857) is an explicit tile escalation/override resolver: direct active providers remain direct; other configurations use legacy bridge/prep fields. It is not necessarily an unrelated provider just because the tile says Cloud.

Direct provider credentials use [credentials.rs](../../../overlay-backend/src/credentials.rs#L1-L26). Windows stores them in Credential Manager; [POSIX implementation](../../../overlay-backend/src/credentials.rs#L132-L217) uses a permissions-restricted JSON file, not encrypted storage. Legacy Groq/bridge/server credentials still exist in Config and portable exports. No source inspection here reads live secrets.

## Requests and streaming

[Protocol wire builder](../../../overlay-backend/src/ai/provider.rs#L14-L188) selects `chat/completions`, `responses` or `messages`; encodes text/image parts, separates system instructions where required, and uses each provider's authorization convention. Unsupported image formats are rejected for Anthropic data-URL conversion. A generic `accepts_images()` flag on AiEndpoint is not a live model-catalog capability proof.

[complete_with_usage_endpoint](../../../overlay-backend/src/ai/completion.rs#L32-L73): Codex dispatches a blocking app-server turn; other protocols resolve managed MLX then enter HTTP completion. [HTTP completion](../../../overlay-backend/src/ai/completion.rs#L136-L275) retries selected transient errors up to three attempts, uses shared client/guards and protocol-specific response parsing. Selected 4xx errors stop retries. Exception text is not a proof of screenshot-safe high-level formatting.

[stream_chat_endpoint](../../../overlay-backend/src/ai/stream.rs#L33-L136) creates a bounded 64-event channel and detached async task. Codex uses its own run_turn branch before the ordinary HTTP semaphore path. HTTP streams acquire one of [two shared permits](../../../overlay-backend/src/ai/control.rs#L7-L15), and exclusive operations acquire both. This limits active HTTP work; it does not universally bound every spawned waiting task, messages allocation or Codex concurrency. Codex callback uses `blocking_send` on that output channel while [its RPC reader](../../../overlay-backend/src/codex_subscription.rs#L158-L182) uses unbounded std mpsc; [tile-spawn bridge](../../../slint-experiment/src/bin/overlay_host_windows.rs#L736-L756) is also unbounded. Intermediate capacity is not end-to-end memory bounding.

[HTTP stream body/response path](../../../overlay-backend/src/ai/stream.rs#L155-L224) logs model/protocol/count/status metadata. [SSE parser](../../../overlay-backend/src/ai/stream.rs#L226-L363) buffers UTF-8 chunks until frame separators, parses provider deltas/terminal markers, stops on closed receiver and emits a fallback Done at transport EOF. It skips unknown/non-JSON frames. Frame-buffer/output byte caps and abrupt EOF semantic correctness require separate tests; no blanket bounded-memory guarantee is claimed.

[Shared client and route guards](../../../overlay-backend/src/ai/control.rs#L16-L128) build a reused client and apply local sampler/no-think/cache plus managed MLX lease/deep-lock checks. Timeouts are explicit per request: [completion 180 seconds](../../../overlay-backend/src/ai/completion.rs#L237-L243), [stream 120 seconds](../../../overlay-backend/src/ai/stream.rs#L198-L204), and token-count 30 seconds. The shared builder itself does not set a default request timeout. Prompt-cache control depends on protocol support; it is not a measured cache-hit or cost-saving guarantee.

## Prompt/context and tile state

[build_request](../../../overlay-backend/src/ai/prompt.rs#L5-L126) frames meeting context and transcript as passive data, grounds KB references (up to three entries/4000 chars) and creates text or image-bearing user parts. Prompt framing is defense-in-depth text, not a security boundary that prevents all model instruction-following attacks.

[AskRoute](../../../slint-experiment/src/bin/overlay_host/tile_routes.rs#L15-L86) selects Text/Vision/Cloud, checks configured endpoint, applies output token cap and stores a per-tile mutable route for sticky escalation. Vision route unavailable yields an empty endpoint rather than silently switching to text. [Main ask](../../../slint-experiment/src/bin/overlay_host/tile_ask.rs#L420-L651) registers a streaming tile, resolves managed runtime when required and sends endpoint-aware streaming work.

[GenGatedEvents](../../../slint-experiment/src/bin/overlay_host/tile_controller.rs#L119-L188) drops stale generations before emitting, but generation check and current-slot use are not one atomic operation. [Delta/terminal handler](../../../slint-experiment/src/bin/overlay_host/tile_controller.rs#L537-L699) schedules Slint UI mutation via Weak handles and keeps conversation history. [Runtime stream loop](../../../overlay-backend/src/runtime.rs#L1810-L1905) journals completion/error/cost with caller-supplied captured context. Closing a receiver, aborting an outer consumer and stopping an in-process native turn are different cancellation boundaries.

## Vision and OCR separation

[Config::vision_endpoint](../../../overlay-backend/src/config.rs#L883-L949) supports same-model capability declaration, legacy cloud/local, direct providers and Codex; unknown/off returns None. The successful local OCR branch needs no vision endpoint and does not send a screenshot to an LLM. This is a branch-level statement, not an unconditional mode guarantee: normal OCR entry initially captures `ep=None` when local OCR is ready, but [launch helper](../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L475-L508) checks availability again. If local engine disappears and a caller supplied Some endpoint, generic vision fallback can be reached; the normal captured-None path instead errors. No actual fallback egress was reproduced.

[Vision helpers](../../../overlay-backend/src/vision.rs#L19-L28) cap answer output at 1536 tokens and JPEG dimensions at 1280px. [Message builder](../../../overlay-backend/src/vision.rs#L128-L224) creates a passive-context image request for Solve/Translate. OCR mode is handled locally by the host, not this vision request.

[Host capture](../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L217-L235) hides/restores own windows around native capture; platform-specific filtering still matters. [Local OCR dispatch](../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L646-L690) runs off-thread and returns text to its placeholder tile. [Vision send](../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L754-L891) captures config/history, resolves MLX as needed, converts image, builds message, streams via per-tile sink and journals `attached_screenshot=true` plus textual context rather than copying raw pixels into journal payload.

## Codex boundary and acceptance limits

[Codex module](../../../overlay-backend/src/codex_subscription.rs#L1-L51) owns official child protocol, isolated home, device-code account state and strict no-tools permission profile. [Turn/type limits](../../../overlay-backend/src/codex_subscription.rs#L23-L36) cap time/delta/output/image/catalog payloads. These policies and fail-closed source branches are not proof of current official CLI compatibility or independently tested token isolation. No direct credential extraction is part of this research.

A review suggestion that every Codex meeting summary is forced through HTTP exclusive completion is **not established by the caller**. [summary_complete](../../../overlay-backend/src/runtime.rs#L574-L599) drops native protocol only when `exclusive=true`, while [caller](../../../overlay-backend/src/runtime.rs#L813-L846) derives that flag from a managed prep session. [ManagedPrepSession](../../../overlay-backend/src/runtime.rs#L445-L470) exists only for app-managed `local` route; Codex route normally gets `exclusive=false` and endpoint-aware completion. HTTP semaphore exclusivity not covering Codex is still a separate true scope limit, not proof of a current bad Codex summary URL.

Source-declared [AI tests](../../../overlay-backend/src/ai/tests.rs) and provider tests cover protocol parsing/body mapping; [route tests](../../../slint-experiment/src/bin/overlay_host/tile_routes.rs#L88-L146) cover auth/capability routing. Required native/provider evidence includes actual account/model catalog, failed/late stream and closed tile behavior, privacy-safe visible failures, stale-generation interleavings and screenshot routing without own-window leakage. See [Grok register](../reconciliation/candidates.json) and [local lifecycle](managed-local-ai.md).
