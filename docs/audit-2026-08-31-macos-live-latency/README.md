# macOS live latency and memory UI audit

## Scope

- Baseline: `d7949618b8dfe35f1b0473fc0900436d4a0bf59b`
- Candidate UI: `a7db6fd78936d6e5c10fdd9096df4cde305ad8dd`
- Follow-up UI: `cfbef936b65cea64ea304f41177407e42f407536`
- Final runtime candidate: `e938a7fd0af6917f6ccdb2cfba87b9d3826bd5ef`
- Capture path: exact candidate built with `--features ui-mcp` and `SLINT_EMIT_DEBUG_INFO=1`, captured through Slint MCP
- Environment: Apple Silicon macOS, Graphite scheme, system display scale, same saved configuration for before/after
- Locales: English and Russian
- Full bar: 1280×64; compact bar: 680×64

## Evidence map

| State | English | Russian |
|---|---|---|
| Baseline full | `before-en-full.png` | `before-ru-full.png` |
| Baseline compact | `before-en-compact.png` | `before-ru-compact.png` |
| MLX loading full | `after-en-loading-full.png` | `after-ru-loading-full.png` |
| MLX ready full | `after-en-ready-full.png` | `after-ru-ready-full.png` |
| MLX ready compact | `after-en-ready-compact.png` | `after-ru-ready-compact.png` |
| Completed request full | `after-en-perf-full.png` | `after-ru-perf-full.png` |
| Completed request compact | `after-en-perf-compact.png` | `after-ru-perf-compact.png` |
| Follow-up memory footer full | `followup-after-en-full.png` | `followup-after-ru-full.png` |
| Follow-up compact regression | `followup-after-en-compact.png` | `followup-after-ru-compact.png` |

## Results

- Immediate background prewarm reached `ready` in both locales without blocking the bar.
- Full ready/request states show `App RAM` / `RAM приложения` and `MLX unified` / `MLX: единая память`; no VRAM wording is used.
- The earliest loading frame may omit MLX memory while no sidecar PID is measurable. This is intentional rather than a fabricated zero value.
- Completed-request captures show model load, first-token latency, total latency, decode tok/s, and end-to-end tok/s in both locales.
- Compact mode intentionally elides memory values while retaining the latency/throughput line without overlap.
- All 14 captures have the expected dimensions. No clipping, overlap, tofu, paths, URLs, usernames, secrets, or private transcript content was observed.
- The Russian baseline already contains the English dynamic labels `recording` and `local:`; the same inherited mixed-language labels remain after the change. The new memory and performance labels themselves are localized.
- Functional macOS hotkey smoke: 13/13 registered actions dispatched. F4 and F7 were retried with explicit key events after the first synthetic dispatch was not observed.

## Bounded production-path benchmark

The approved three-cell run used `overlay_backend::ai::stream_chat_endpoint` and the managed MLX lifecycle, then stopped without tuning:

| Cell | Load | TTFT | Total | Decode | End-to-end | MLX memory |
|---|---:|---:|---:|---:|---:|---:|
| Cold | 34.808 s | unavailable | 37.527 s | 89.663 tok/s | 2.558 tok/s | 4.71 GiB |
| Warm | resident | unavailable | 1.227 s | 87.709 tok/s | 78.196 tok/s | 4.71 GiB |
| Long synthetic context | resident | unavailable | 9.992 s | 79.884 tok/s | 9.607 tok/s | 6.09 GiB |

The benchmark stream reported completion-token metrics but no visible content delta, so TTFT is recorded as unavailable rather than inferred. Total GPU-active benchmark time was 50 seconds, below the approved 15-minute ceiling.

## Follow-up auto-tile output-budget screen

A bounded follow-up used the installed native Swift sidecar and its production `/v1/chat/completions` streaming API with the pinned LFM model, production chat template, temperature `0.2`, and one fixed concise-answer prompt. Python was used only as the benchmark HTTP harness; the measured inference process remained the Swift/Metal sidecar.

| Max output tokens | Visible characters | TTFT | Total | Finish |
|---:|---:|---:|---:|---|
| 512 | 0 | unavailable | 6.034 s | length |
| 1024 | 509 | 5.835 s | 7.034 s | stop |
| 2048 | 451 | 6.096 s | 7.237 s | stop |
| 4096 | 481 | 9.659 s | 11.523 s | stop |

The 1024-token cell was the smallest successful budget and the fastest successful total in this screen, but a later exact-candidate system-audio smoke produced an empty, length-limited 1024-token answer for a real detected question. A second exact-candidate two-question smoke at 2048 still produced one empty, length-limited answer. The final auto-tile path therefore uses the already measured 4096 budget, matching the manual ask path; that cell completed visibly in 11.523 seconds. This single-prompt screen plus the later functional smokes establish truncation handling for the selected production path; they do not claim general model quality or load capacity. Total measured generation time was about 32 seconds and remained below the approved 15-minute GPU ceiling.

## Follow-up memory-footer audit

The exact follow-up UI candidate `cfbef936b65cea64ea304f41177407e42f407536` was captured in English and Russian at 1280×64 and 680×64. A bounded synthetic loopback sidecar was used only to populate deterministic process-footprint values during these layout captures; it did not generate model output or replace the separately built and tested native Swift sidecar.

- Full mode places the model/load line first and `App RAM` / `RAM приложения` plus the short `MLX` label on a separate line underneath.
- The English capture shows `App RAM 328 MiB` and `MLX 171 MiB`; Russian shows `RAM приложения 327 MiB` and `MLX 171 MiB`.
- Both compact captures retain the pre-existing single-line loading state without clipping or overlap.
- No paths, URLs, usernames, credentials, private transcripts, tofu, clipping, or overlap were observed.
- The 13-key hotkey matrix was not repeated for this follow-up because no hotkey code changed; the earlier 13/13 functional evidence above remains the relevant hotkey result.

## Final live functional smoke

The stable-signed runtime candidate `e938a7fd0af6917f6ccdb2cfba87b9d3826bd5ef` was installed over the existing app and exercised through real macOS system-audio capture with two spoken questions. Startup armed system capture without opening a competing standalone Core Audio probe.

- 2 system transcript lines produced 2 detector triggers and 2 auto-tile requests.
- Both responses contained visible text and completed with `finish_reason=stop`.
- 2 answer tiles spawned; the journal recorded no errors.
- No Python process participated in production inference; the resident model process was the native Swift/Metal sidecar.
